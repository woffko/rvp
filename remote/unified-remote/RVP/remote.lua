-- RVP's remote for Unified Remote (M15). It controls RVP through RVP's
-- control API on this computer (http://127.0.0.1:PORT, which RVP's Remote
-- Control settings page turns on), with RVP's token on every request.
--
-- Settings (settings.prop, editable in Unified Remote's manager): port,
-- RVP's port (48931 unless the page changed it), and token. With an empty
-- token the remote reads the one an installed RVP keeps in
-- %APPDATA%\RVP\remote-token (Windows) or $XDG_STATE_HOME/rvp/remote-token
-- (Linux, ~/.local/state/rvp); a portable RVP's token is copied from its
-- Remote Control page.
--
-- Every action RVP carries out is named as in RVP's own list of actions
-- (GET /api/actions); crates/app/tests/remote.rs checks that each exists.

local http = libs.http
local data = libs.data
local server = libs.server
local timer = libs.timer
local fs = libs.fs

-- RVP's last status, the status timer while the remote shows, and what the
-- Albums page lists: the albums (album = nil) or one album's entries.
local status = nil
local poll = nil
local album = nil
local listed = {}

local update

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

local function token()
	local own = settings.token
	if own ~= nil and own ~= "" then
		return own
	end
	local path = state_folder() .. (OS_WINDOWS and "\\" or "/") .. "remote-token"
	if fs.exists(path) then
		return (string.gsub(fs.read(path), "%s", ""))
	end
	return ""
end

local function base()
	local port = settings.port
	if port == nil or port == "" then
		port = "48931"
	end
	return "http://127.0.0.1:" .. port
end

-- One request to RVP's control API: its answer, or nil (the title says
-- why).
local function call(method, path, body)
	local request = {
		method = method,
		url = base() .. path,
		headers = { Authorization = "Bearer " .. token() }
	}
	if body ~= nil then
		request.mime = "application/json"
		request.content = data.tojson(body)
	end
	local ok, response = pcall(http.request, request)
	local code = nil
	if ok and response ~= nil then
		code = tonumber(response.status)
	end
	if code ~= 200 then
		local why = "RVP does not answer: turn on Remote Control in RVP's settings"
		if code == 401 then
			why = "RVP refused the token: copy it from RVP's Remote Control page into this remote's settings"
		elseif code ~= nil then
			why = "RVP answered " .. code
		end
		server.update({ id = "title", text = why })
		return nil
	end
	if response.content == nil or response.content == "" then
		return {}
	end
	return data.fromjson(response.content)
end

-- Runs one of RVP's actions, as its keys and menus do.
local function act(name)
	call("post", "/api/action/" .. name)
	update()
end

local function span(seconds)
	return data.sec2span(math.floor(seconds or 0))
end

local function selected(tracks, none)
	for _, track in ipairs(tracks or {}) do
		if track.selected then
			return track.title
		end
	end
	return none
end

update = function ()
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
		{ id = "normalizer", checked = now.normalizer == true },
		{ id = "tracks", text = "Audio: " .. selected(now.audio_tracks, "none")
		  .. "\nSubtitles: " .. selected(now.subtitle_tracks, "off") }
	)
end

-- The status once a second while the remote shows; the next request
-- waits for the last one.
local function tick()
	update()
	poll = timer.timeout(tick, 1000)
end

events.detect = function ()
	return fs.exists(config_folder())
end

events.focus = function ()
	tick()
end

events.blur = function ()
	if poll ~= nil then
		timer.cancel(poll)
		poll = nil
	end
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
	local shown = call("get", "/api/albums/" .. index)
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
		call("post", "/api/albums/" .. album .. "/play/" .. target)
		update()
	end
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
