-- RVP's remote for Unified Remote (M15). It controls RVP through RVP's
-- control API on this computer (http://127.0.0.1:PORT, which RVP's Remote
-- Control settings page turns on). The token never leaves this computer's
-- remote: each request asks RVP for a nonce and carries a signature over
-- it, made with the token (the protocol is in RVP's
-- crates/app/src/control/mod.rs), so a program that took the port while RVP
-- was off learns nothing it could use against RVP.
--
-- Settings (settings.prop, editable in Unified Remote's manager): port,
-- RVP's port (48931 unless the page changed it), token, and token_file.
-- With an empty token the remote reads the file token_file names, which
-- RVP's Add to Unified Remote (its Remote Control page) sets to its own,
-- a portable RVP's too; with neither, the one an installed RVP keeps in
-- %APPDATA%\RVP\remote-token (Windows) or $XDG_STATE_HOME/rvp/remote-token
-- (Linux, ~/.local/state/rvp).
--
-- Every action RVP carries out is named as in RVP's own list of actions
-- (GET /api/actions); crates/app/tests/remote.rs checks that each exists.

local http = libs.http
local data = libs.data
local server = libs.server
local timer = libs.timer
local fs = libs.fs

-- RVP's last status, the status timer while the remote shows and the
-- number of its chain, and what the Albums page lists: the albums
-- (album = nil) or one album's entries.
local status = nil
local poll = nil
local chain = 0
local album = nil
local listed = {}
-- What each row of the Tracks page's list chooses (POST /api/track's
-- body), and the list's text as last sent (sent again when it changes).
local track_rows = {}
local tracks_shown = nil

local update

-- SHA-256 (FIPS 180-4) and HMAC (RFC 2104) in plain Lua, written for RVP:
-- no bit library and no integer operators, so that the same arithmetic
-- runs on Lua 5.1 to 5.4. A word is a number from 0 to 2^32 - 1; XOR goes
-- through a table of bytes, AND follows from it (a + b = (a XOR b) +
-- 2 (a AND b)), and shifts are products and quotients of powers of two.
-- crates/app/tests/remote.rs runs it against the standard test vectors and
-- RVP's own signatures.

local WORD = 4294967296

-- XOR of two bytes a and b: XOR8[a * 256 + b + 1]; each from the one of
-- their upper seven bits, which comes earlier in the loop.
local XOR8 = { 0 }
for a = 0, 255 do
	for b = 0, 255 do
		if a + b > 0 then
			local high = XOR8[(a - a % 2) / 2 * 256 + (b - b % 2) / 2 + 1]
			XOR8[a * 256 + b + 1] = high * 2 + (a + b) % 2
		end
	end
end

local function bxor(a, b)
	local out, place = 0, 1
	for _ = 1, 4 do
		local x, y = a % 256, b % 256
		out = out + XOR8[x * 256 + y + 1] * place
		a, b, place = (a - x) / 256, (b - y) / 256, place * 256
	end
	return out
end

local function band(a, b)
	return (a + b - bxor(a, b)) / 2
end

-- Rotates right by n bits.
local function rotr(x, n)
	local low = x % 2 ^ n
	return low * 2 ^ (32 - n) + (x - low) / 2 ^ n
end

local function shr(x, n)
	return (x - x % 2 ^ n) / 2 ^ n
end

local K = {
	0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
	0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
	0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
	0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
	0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
	0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
	0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
	0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
}

-- The four bytes of a word, most significant first.
local function word_bytes(w)
	local b4 = w % 256
	w = (w - b4) / 256
	local b3 = w % 256
	w = (w - b3) / 256
	local b2 = w % 256
	return string.char((w - b2) / 256, b2, b3, b4)
end

-- One 64-byte block of `message` from `at` into the state h.
local function compress(h, message, at)
	local w = {}
	for i = 1, 16 do
		local b1, b2, b3, b4 = string.byte(message, at + i * 4 - 4, at + i * 4 - 1)
		w[i] = ((b1 * 256 + b2) * 256 + b3) * 256 + b4
	end
	for i = 17, 64 do
		local x, y = w[i - 15], w[i - 2]
		local s0 = bxor(bxor(rotr(x, 7), rotr(x, 18)), shr(x, 3))
		local s1 = bxor(bxor(rotr(y, 17), rotr(y, 19)), shr(y, 10))
		w[i] = (w[i - 16] + s0 + w[i - 7] + s1) % WORD
	end
	local a, b, c, d, e, f, g, hh = h[1], h[2], h[3], h[4], h[5], h[6], h[7], h[8]
	for i = 1, 64 do
		local s1 = bxor(bxor(rotr(e, 6), rotr(e, 11)), rotr(e, 25))
		-- Ch(e, f, g) = g XOR (e AND (f XOR g))
		local ch = bxor(g, band(e, bxor(f, g)))
		local t1 = (hh + s1 + ch + K[i] + w[i]) % WORD
		local s0 = bxor(bxor(rotr(a, 2), rotr(a, 13)), rotr(a, 22))
		-- Maj(a, b, c) = (a AND (b XOR c)) XOR (b AND c)
		local maj = bxor(band(a, bxor(b, c)), band(b, c))
		local t2 = (s0 + maj) % WORD
		hh, g, f, e, d, c, b, a = g, f, e, (d + t1) % WORD, c, b, a, (t1 + t2) % WORD
	end
	h[1], h[2], h[3], h[4] = (h[1] + a) % WORD, (h[2] + b) % WORD, (h[3] + c) % WORD, (h[4] + d) % WORD
	h[5], h[6], h[7], h[8] = (h[5] + e) % WORD, (h[6] + f) % WORD, (h[7] + g) % WORD, (h[8] + hh) % WORD
end

-- SHA-256 of a string: its 32 bytes.
local function sha256(message)
	local bits = #message * 8
	message = message .. "\128" .. string.rep("\0", (55 - #message) % 64)
		.. word_bytes((bits - bits % WORD) / WORD) .. word_bytes(bits % WORD)
	local h = {
		0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
	}
	for at = 1, #message, 64 do
		compress(h, message, at)
	end
	local out = {}
	for i = 1, 8 do
		out[i] = word_bytes(h[i])
	end
	return table.concat(out)
end

local function to_hex(bytes)
	local out = {}
	for i = 1, #bytes do
		out[i] = string.format("%02x", string.byte(bytes, i))
	end
	return table.concat(out)
end

-- HMAC-SHA-256 of `message` keyed with `key`: its 32 bytes.
local function hmac_sha256(key, message)
	if #key > 64 then
		key = sha256(key)
	end
	key = key .. string.rep("\0", 64 - #key)
	local inner, outer = {}, {}
	for i = 1, 64 do
		local byte = string.byte(key, i)
		inner[i] = string.char(XOR8[byte * 256 + 0x36 + 1])
		outer[i] = string.char(XOR8[byte * 256 + 0x5c + 1])
	end
	return sha256(table.concat(outer) .. sha256(table.concat(inner) .. message))
end

-- The Authorization value of a signed request: RVP's scheme, the nonce
-- and the signature over the port, the nonce, the method, the target and
-- the body.
local SCHEME = "RVP-HMAC-SHA256"

local function signed(key, port, nonce, method, target, body)
	local text = SCHEME .. "\n" .. port .. "\n" .. nonce .. "\n" .. method .. "\n" .. target .. "\n" .. body
	return SCHEME .. " nonce=" .. nonce .. ", signature=" .. to_hex(hmac_sha256(key, text))
end

-- crates/app/tests/remote.rs runs the remote with a Lua of its own, which
-- sets rvp_check to reach these; Unified Remote does not.
if rvp_check ~= nil then
	rvp_check.sha256 = function (message) return to_hex(sha256(message)) end
	rvp_check.hmac = function (key, message) return to_hex(hmac_sha256(key, message)) end
end

-- Where an installed RVP keeps its state and its settings.
local function state_folder()
	if OS_WINDOWS then
		return "%APPDATA%\\RVP"
	end
	local state = os.getenv("XDG_STATE_HOME")
	if state == nil or state == "" then
		state = (os.getenv("HOME") or "") .. "/.local/state"
	end
	return state .. "/rvp"
end

local function config_folder()
	if OS_WINDOWS then
		return "%APPDATA%\\RVP"
	end
	local config = os.getenv("XDG_CONFIG_HOME")
	if config == nil or config == "" then
		config = (os.getenv("HOME") or "") .. "/.config"
	end
	return config .. "/rvp"
end

-- The token's file: the one the setting token_file names, else an
-- installed RVP's.
local function token_file()
	local named = settings.token_file
	if named ~= nil and named ~= "" then
		return named
	end
	return state_folder() .. (OS_WINDOWS and "\\" or "/") .. "remote-token"
end

-- The token: the remote's setting, else its file ("" when there is none
-- or it cannot be read).
local function token()
	local own = settings.token
	if own ~= nil and own ~= "" then
		return (string.gsub(own, "%s", ""))
	end
	local path = token_file()
	if fs.exists(path) then
		local ok, text = pcall(fs.read, path)
		if ok and type(text) == "string" then
			return (string.gsub(text, "%s", ""))
		end
	end
	return ""
end

-- A whole number as text (Lua 5.3 and later write a float's ".0").
local function whole(n)
	return string.format("%d", n)
end

-- RVP's port: the setting when it is a whole number from 1 to 65535, else
-- RVP's default.
local function port()
	local n = tonumber(settings.port)
	if n == nil or n ~= math.floor(n) or n < 1 or n > 65535 then
		return 48931
	end
	return n
end

-- One exchange with whatever listens on RVP's port: the answer's status
-- (nil when nothing answered) and its JSON (nil when it is none).
local function exchange(method, target, content, authorization)
	local request = { method = method, url = "http://127.0.0.1:" .. whole(port()) .. target }
	if authorization ~= nil then
		request.headers = { Authorization = authorization }
	end
	if content ~= nil then
		request.mime = "application/json"
		request.content = content
	end
	local ok, response = pcall(http.request, request)
	if not ok or response == nil then
		return nil, nil
	end
	local code = tonumber(response.status)
	if response.content == nil or response.content == "" then
		return code, {}
	end
	local parsed, value = pcall(data.fromjson, response.content)
	if not parsed then
		return code, nil
	end
	return code, value
end

-- A nonce of RVP's: 32 hex digits, nothing else.
local function nonce()
	local code, answer = exchange("get", "/api/nonce")
	if code ~= 200 or type(answer) ~= "table" or type(answer.nonce) ~= "string" then
		return nil, code
	end
	if #answer.nonce ~= 32 or string.find(answer.nonce, "[^0-9a-f]") ~= nil then
		return nil, code
	end
	return answer.nonce, code
end

-- One signed request to RVP's control API: its answer, or nil (the title
-- says why).
local function call(method, target, body)
	local content = nil
	if body ~= nil then
		content = data.tojson(body)
	end
	local fresh, code = nonce()
	local value = nil
	if fresh ~= nil then
		local authorization = signed(token(), whole(port()), fresh, string.upper(method), target, content or "")
		code, value = exchange(method, target, content, authorization)
	elseif code == 200 then
		code = nil
	end
	if code ~= 200 or value == nil then
		local why = "RVP does not answer: turn on Remote Control in RVP's settings"
		if code == 401 then
			why = "RVP refused the token: copy it from RVP's Remote Control page into this remote's settings"
		elseif code ~= nil and code ~= 200 then
			why = "RVP answered " .. whole(code)
		end
		server.update({ id = "title", text = why })
		return nil
	end
	return value
end

-- Runs one of RVP's actions, as its keys and menus do.
local function act(name)
	call("post", "/api/action/" .. name)
	update()
end

local function span(seconds)
	return data.sec2span(math.floor(seconds or 0))
end

-- A row of the Tracks page: the kind, the track's title (with its
-- language where the title is another), "> " before the one that plays.
local function track_text(kind, track)
	local text = track.title or ("Track " .. whole(track.id))
	if track.language ~= nil and track.language ~= "" and track.language ~= track.title then
		text = text .. " [" .. track.language .. "]"
	end
	if track.selected then
		text = "> " .. text
	end
	return kind .. ": " .. text
end

-- The Tracks page's list: the file's audio tracks, its subtitles and
-- "Subtitles: off"; a tap plays the row's (actions.track_tapped).
local function show_tracks(now)
	local rows, items, texts, shown = {}, {}, {}, false
	for _, track in ipairs(now.audio_tracks or {}) do
		table.insert(items, { type = "item", text = track_text("Audio", track) })
		table.insert(rows, { audio = track.id })
	end
	for _, track in ipairs(now.subtitle_tracks or {}) do
		if track.selected then
			shown = true
		end
		table.insert(items, { type = "item", text = track_text("Subtitles", track) })
		table.insert(rows, { subtitle = track.id })
	end
	if #(now.subtitle_tracks or {}) > 0 then
		local off = "Subtitles: off"
		if not shown then
			off = "> " .. off
		end
		table.insert(items, { type = "item", text = off })
		table.insert(rows, { subtitle = false })
	end
	if #items == 0 then
		table.insert(items, { type = "item", text = "No tracks" })
	end
	for _, item in ipairs(items) do
		table.insert(texts, item.text)
	end
	local key = table.concat(texts, "\n")
	if key ~= tracks_shown then
		tracks_shown = key
		track_rows = rows
		server.update({ id = "tracklist", children = items })
	end
end

local function show_status()
	local now = call("get", "/api/status")
	if now == nil then
		return
	end
	status = now
	local title = now.title
	if title == nil or title == "" then
		title = "RVP: nothing plays"
	end
	local length = math.max(1, math.floor(now.duration or 0))
	local icon = "play"
	if now.state == "playing" then
		icon = "pause"
	end
	local volume = math.floor((now.volume or 0) + 0.5)
	server.update(
		{ id = "title", text = title },
		{ id = "position", progress = math.floor(now.position or 0), progressmax = length,
		  text = span(now.position) .. " / " .. span(now.duration) },
		-- Unified Remote writes "progress / progressmax" in percent beside the
		-- text, so the slider spans RVP's 0 to 100 % (as RVP's skins' volume
		-- sliders do; above 100 % it stays full and the percent goes on).
		{ id = "volume", progress = volume, progressmax = 100 },
		{ id = "play", icon = icon },
		{ id = "speed", text = string.format("%.2fx", now.speed or 1) },
		{ id = "normalizer", checked = now.normalizer == true }
	)
	show_tracks(now)
end

-- Shows RVP's status; an answer the remote cannot show (another program
-- on the port) says so in the title instead of stopping the caller.
update = function ()
	if not pcall(show_status) then
		pcall(server.update, { id = "title", text = "RVP does not answer: turn on Remote Control in RVP's settings" })
	end
end

-- The status once a second while the remote shows; the next request
-- waits for the last one. Each focus starts a chain of its own number and
-- ends the one before, also when its timer was not cancelled.
local function tick(mine)
	if mine ~= chain then
		return
	end
	update()
	poll = timer.timeout(function () tick(mine) end, 1000)
end

local function stop_polling()
	chain = chain + 1
	if poll ~= nil then
		timer.cancel(poll)
		poll = nil
	end
end

events.detect = function ()
	return fs.exists(config_folder()) or fs.exists(token_file())
end

events.focus = function ()
	stop_polling()
	tick(chain)
end

events.blur = function ()
	stop_polling()
end

local function show_albums()
	local albums = call("get", "/api/albums")
	if albums == nil then
		return
	end
	album = nil
	listed = {}
	local items = {}
	for _, each in ipairs(albums) do
		local mark = ""
		if each.playing then
			mark = "> "
		end
		table.insert(items, { type = "item", text = mark .. each.name .. "\n" .. each.entries .. " entries" })
		table.insert(listed, each.index)
	end
	server.update({ id = "where", text = "Albums" }, { id = "list", children = items })
end

local function show_album(index)
	local shown = call("get", "/api/albums/" .. whole(index))
	if shown == nil then
		return
	end
	album = index
	listed = {}
	local items = {}
	for _, entry in ipairs(shown.entries or {}) do
		local mark = ""
		if shown.playing and shown.current == entry.index then
			mark = "> "
		end
		local text = mark .. entry.title
		if entry.duration ~= nil then
			text = text .. "\n" .. span(entry.duration)
		end
		table.insert(items, { type = "item", text = text })
		table.insert(listed, entry.index)
	end
	server.update({ id = "where", text = shown.name }, { id = "list", children = items })
end

--@help Play or pause
actions.play_pause = function ()
	act("play-pause")
end

--@help Close playback
actions.stop = function ()
	act("close-media")
end

--@help Previous file
actions.previous = function ()
	act("previous")
end

--@help Next file
actions.next = function ()
	act("next")
end

--@help Jump back (RVP's short step, 5 s unless changed)
actions.back = function ()
	act("seek-backward-short")
end

--@help Jump forward (RVP's short step, 5 s unless changed)
actions.forward = function ()
	act("seek-forward-short")
end

--@help Volume up
actions.volume_up = function ()
	act("volume-up")
end

--@help Volume down
actions.volume_down = function ()
	act("volume-down")
end

--@help Mute on or off
actions.mute = function ()
	act("toggle-mute")
end

--@help Fullscreen on or off
actions.fullscreen = function ()
	act("toggle-fullscreen")
end

--@help Next audio track
actions.audio_track = function ()
	act("cycle-audio")
end

--@help Next subtitle track
actions.subtitle_track = function ()
	act("cycle-subtitles")
end

--@help Subtitles on or off
actions.subtitles_on_off = function ()
	act("toggle-subtitles")
end

--@help Slower
actions.slower = function ()
	act("speed-down")
end

--@help Faster
actions.faster = function ()
	act("speed-up")
end

--@help Normal speed
actions.speed_reset = function ()
	act("speed-default")
end

--@help Normalizer on or off (the toggle's state)
actions.normalizer = function (checked)
	if status == nil or status.normalizer ~= checked then
		act("toggle-normalizer")
	end
end

--@help Go to a time
--@param seconds:number seconds from the start
actions.seek = function (seconds)
	call("post", "/api/seek", { to = tonumber(seconds) })
	update()
end

--@help Set the volume
--@param percent:number RVP's volume in percent
actions.volume = function (percent)
	call("post", "/api/volume", { percent = tonumber(percent) })
end

--@help The albums page
actions.tab_changed = function (index)
	if index == 2 then
		show_albums()
	end
end

--@help Open an album, or play an entry of the open album
--@param index:number the row
actions.list_tapped = function (index)
	local target = listed[index + 1]
	if target == nil then
		return
	end
	if album == nil then
		show_album(target)
	else
		call("post", "/api/albums/" .. whole(album) .. "/play/" .. whole(target))
		update()
	end
end

--@help Play the tapped audio or subtitle track
--@param index:number the row
actions.track_tapped = function (index)
	local row = track_rows[index + 1]
	if row == nil then
		return
	end
	call("post", "/api/track", row)
	update()
end

--@help Back to the albums
actions.list_back = function ()
	show_albums()
end

--@help Read the list again
actions.list_refresh = function ()
	if album == nil then
		show_albums()
	else
		show_album(album)
	end
end
