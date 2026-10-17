# M11 studies: seamless playback and the file navigator

Goal M asked to study PotPlayer's seamless playback and its file navigator
and Open Box, to record the decision, and to add them only if they keep
the album rules (mpv plays one entry at a time, AGENTS.md) and the GUI
thread free. Both were studied on 2026-10-17; neither went in. Their
menu items and Preferences options keep these reasons
(`crates/app/src/coverage.rs`, `docs/coverage/`).

## Seamless playback

PotPlayer's Playback > Seamless Playback offers "Preload Multiple Files"
(the next file opens before the current one ends, so the switch has no
gap) and "Preload & Combine Multiple Files" (several files play as one
timeline), with a page of settings (method, files to process, preloading
before playback starts).

What libmpv offers:

- `prefetch-playlist` opens the next entry of mpv's own playlist early
  (demuxer and cache), and `gapless-audio` keeps the audio output running
  across files of one format. Both work only inside mpv's playlist.
- An EDL address (`edl://a.mkv;b.mkv`) joins files into one timeline.

Why neither keeps the album rules:

1. A two-entry mpv playlist (the current entry and the next, appended
   while the current one plays) lets mpv move on by itself at the end of a
   file. RVP decides there: Repeat Playback, the queue, shuffle, Skip
   Ending, "When Playback Finished" (exit, shut down after a countdown,
   stay), the Recycle Bin's question for played files and the resume
   positions all act at that moment, and the album may have changed since
   the next entry was appended (sorting, the queue, shuffle switched on).
   mpv moving on first would race with all of them; this is the reason the
   rule exists (goal E, goal I's end-of-file fixes).
2. An EDL join makes several entries one mpv file: each entry's position,
   duration, title, track choices, bookmarks and A-B list, and the album's
   current entry, would be lost or wrong, and seeks would cross files.
3. A second core preloading the next file cannot hand its demuxer or
   decoder to the player's core: libmpv has no API for it.

What the switch costs today: RVP keeps the last frame (`keep-open`) and
loads the next entry with `loadfile replace` as soon as it sees the end.
`crates/app/tests/m11.rs` (`the_next_entry_follows_the_end_of_a_file`)
measures, headless, the time from mpv's end of one fixture to the next
entry's first frame: 68 to 74 ms in WSL (local files) and 85 to 119 ms
on the Windows test VM. A window's video output and the audio device add their
own start on top.
The last frame stays on screen meanwhile, so the gap shows as a short
hold of the picture and a pause in the sound.

Decision: seamless playback is not added. If the owner wants gapless
albums later, the way that keeps the rule is mpv's `gapless-audio` for
audio-only albums with RVP still choosing the next entry; it needs mpv's
playlist and is therefore an owner decision, not part of M11.

## The file navigator and Open Box

PotPlayer's file navigator (Open > Navigate, F12) is a file browser of its
own: a navigation pane (folders, favorites, albums, URLs, FTP and WebDAV
servers, bookmarks, history, scenes) and a content pane (a list or video
thumbnails, a search box), with about forty options on the Playlist >
Color/View page. Open Box is its launcher form.

- Album rules: a navigator only opens files, so it could keep them (the
  files it opens would join the shown album as Open File(s) does).
- GUI thread: folder listings (network shares can take seconds) and
  thumbnails would have to come from threads; the thumbnail core
  (`crates/engine/src/thumbnail.rs`) could serve them.
- Cost and value: it is a second file manager in QML, about the size of
  the settings window, for ways in that RVP already has: the system's file
  and folder dialogs, Open Folder and Add Folder, Album/Favorites (the
  favorites and the recent items), the bookmark list and the scene
  browsers, and the albums of the playlist panel.

Decision: the navigator and Open Box are not added in M11. Their items
(Open > Navigate, Default Open Action > Open File Navigator, `/simple`)
and the Color/View options keep the reason. Whether RVP should grow one is
listed for the owner in `docs/goals/goal-m-status.md`.
