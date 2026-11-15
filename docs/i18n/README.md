# RVP's interface languages

RVP's interface is written in English, the source text. Every other
language is a translation written for RVP (D20, M19): its files in
`shell/i18n`, and its rules and glossary in this folder
(`docs/i18n/<code>.md`). No language but English is written into RVP's
code; adding a language is adding its files.

## The files

| File | What | Made by |
|------|------|---------|
| `shell/i18n/qml_<code>.ts` | the QML files' strings (windows, settings, dialogs) | lupdate, then translated |
| `shell/i18n/menu_<code>.ts` | the menu and action labels, context "Menu" (`Tools.tr_menu`; numbers stand aside as `%1`, `%2`...) | RVP's own list, translated |
| `shell/i18n/packaging_<code>.ts` | the desktop entry's GenericName and Comment, the Windows installer's strings | RVP's own list, translated |
| `docs/i18n/<code>.md` | the language's rules and glossary | written before its translation |

Each `.ts` message has the English `source`, often an `extracomment`
that says where it shows, and the `translation`. The Russian files are
complete and serve as a second reference for meaning.

Checks: `tests/translations.rs` (every string translated in every
language, placeholders kept, the language's own name, the desktop
entry), `rvp --selftest=language --language=<code>` (the language loads,
and every text of the settings window fits its control),
`tools/screenshots/tour.sh` with `RVP_TOUR_LANGUAGE=<code>` (pictures of
the main views to look through).

## Rules for every language

- **Translate the meaning, for this program.** Read the comment and the
  context's other strings; the Russian translation shows how an earlier
  translator read the English. Say what the control does in the words a
  user of this language expects from a desktop media player.
- **Our own words.** Never copy another program's translations
  (PotPlayer's, VLC's, mpv's, the desktops'); established terms (the
  usual word for "playlist" or "subtitles") are the language's, not a
  program's, and are welcome.
- **Names stay as they are:** RVP, RVP Default, PotPlayer, BlackSpace,
  SVP, SVP 4 Manager, mpv, libmpv, FFmpeg, VapourSynth, AviSynth,
  MVTools, RIFE, TensorRT, CUDA, NVIDIA, NVIDIA Optical Flow, AMD,
  Vulkan, OpenGL, Direct3D, DirectShow, Qt, Unified Remote, whisper.cpp,
  yt-dlp, Winamp, VLC, MPRIS, PulseAudio, PipeWire, GNOME, KDE,
  Windows, Linux, Wayland, X11, DVD, Blu-ray, BD-J, HDR, SDR, NVDEC,
  dav1d, TomSteady, Crystality, Freeverb, codec and format names (H.264,
  HEVC, AV1, AAC, AC3, DTS, MP3, SRT, ASS, WebVTT, PNG, JPEG...), file
  names, paths and settings keys.
- **Placeholders** `%1`...`%9` stay, each exactly once where the source
  has it; their order may change to suit the sentence.
- **Keys** keep the names RVP shows in its menus: Ctrl, Shift, Alt,
  Enter, Space, Esc, Tab, F1...F12, PgUp, PgDn, Home, End and the
  letters, as in the source.
- **"..."** at the end of a menu item or button (a dialog follows) stays
  three periods, as in the source.
- **"&"** is a literal ampersand (RVP's menus show it as it is): use the
  language's own "and" or keep "&", never a mnemonic.
- **Units:** seconds, minutes and similar take the language's usual
  abbreviation; dB, Hz, kHz, fps, px, %, K (resolution), ms and the like
  stay as written unless the language customarily writes them
  otherwise (the language's file says).
- **Length.** Menu items, buttons, checkboxes, choices and other short
  labels stay about as long as the English: about a third longer at
  most, counting a Chinese, Japanese or Korean character as two Latin
  letters. Help texts (whole sentences under a page's options) may grow.
  Shorter is better whenever the meaning stays.
- **Consistency.** A glossary term is translated the same way everywhere;
  the same English label gets the same translation in every context.
- **Plain text.** No markup, no line breaks the source lacks.

## How a language is made

The goal's agents follow these steps; `tools/i18n/work.py` prepares and
collects their files in a work folder outside the repository.

1. **Rules and glossary.** The translator writes `docs/i18n/<code>.md`:
   the form of address, how labels are phrased, capitalization,
   punctuation and quotation marks, units, and the glossary below with
   the language's term for each (with a note where a term has a trap).
2. **Translation.** `work.py prepare <code> <dir>` writes the strings in
   chunks (`in-NN.json`: id, part, context, source, comment, the Russian);
   the translator answers each with `out-NN.json` (id -> translation),
   following its rules file. `work.py assemble <code> <dir>` writes the
   language's `.ts` files.
3. **Review.** Another agent, which did not translate, gets
   `work.py back <code> <dir>`'s chunks (id, context, comment,
   translation: no English, no Russian) and translates every string back
   into English (`back-out-NN.json`). `work.py compare <code> <dir>`
   puts source, translation and back-translation side by side and flags
   placeholders, English left untranslated, glossary terms not used and
   lengths (`compare-NN.json`). The reviewer reads every row and lists
   the strings whose meaning drifted, whose terms or register break the
   rules, or that read badly (`issues.json`: id, problem, proposal).
4. **Fixes.** The translator answers the issues (`fixes.json`: id ->
   translation, or keeps a string and says why); `work.py assemble`
   takes the fixes; the reviewer looks again at the fixed strings only.
5. **Fit and look.** `--selftest=language --language=<code>` under Xvfb
   and the screenshot tour in the language; overflowing texts get
   shorter.

## Glossary

Each language's file translates every term (its "Glossary" table:
English, the language's term, a note). The meaning is RVP's.

| Term | Meaning in RVP |
|------|----------------|
| play / pause / stop | the transport commands |
| playback | playing a file (the Playback menu and pages) |
| close playback | stop and unload the file |
| previous / next | the previous or next entry of the playlist |
| seek | move the position in the file |
| jump | go to a given time, chapter or subtitle |
| frame step | show the next or previous frame while paused |
| speed | the playback rate (1.0x) |
| repeat | play again: a file, the playlist |
| A-B repeat / section repeat | play a part between two marks again and again |
| shuffle | play the entries in random order |
| queue | entries marked to play next |
| resume / remember position | start a file where it stopped |
| position | the current time in the file |
| duration | a file's whole length |
| elapsed / remaining time | time played / time left |
| chapter | a named part of a file |
| bookmark | a position the user saved |
| skip (intro, ending) | pass over a file's opening or closing part |
| seek bar | the bar showing and setting the position |
| thumbnail | a small picture of a frame (seek bar, playlist) |
| preview | a picture shown while pointing at the seek bar |
| scene browser | a window of frames across the file |
| file | a media file |
| folder | a directory |
| URL / address | a network address to open |
| stream | network media or a track inside a file |
| device / capture device | a webcam or capture card |
| screen capture | the desktop as a video source |
| disc | a DVD, Blu-ray or audio CD |
| title | a disc's main unit (a DVD title) |
| angle | a DVD's alternative camera view |
| disc menu | a DVD or Blu-ray's own menu |
| drag and drop | dropping files onto the window |
| recent files | the files opened lately |
| favorites | the user's saved places and files |
| file association / file type | RVP registered for a type of file |
| playlist | the list of entries to play |
| album | one of the playlist's tabs (a named list) |
| entry | one item of the playlist |
| sort | order the playlist's entries |
| add / remove / delete | to the playlist / from it / the file itself |
| recycle bin / trash | where deleted files go |
| import / export | read or write a file of settings or playlists |
| window / main window | RVP's windows |
| fullscreen / windowed | the window filling the screen / not |
| maximize / minimize | the window's states |
| always on top / stay on top | the window above other windows |
| opacity | how transparent a window is |
| skin | the window's look, PotPlayer's `.dsf` files and others |
| color theme | a skin's color variant |
| logo | the picture shown without a video |
| title bar | the window's top bar |
| control panel | the F7 window of quick settings |
| OSD / on-screen display | messages drawn over the video |
| on-screen controller | mpv's controls drawn over the video |
| tooltip | the text shown on pointing |
| context menu | the right-click menu |
| tray icon / notification area | the icon by the clock |
| taskbar | Windows' bar of windows; the dock |
| notification | a message from the system |
| dialog | a small window asking something |
| settings / preferences | the settings window (F5) |
| page | one page of the settings window |
| preset / configuration | a saved set of settings |
| reset / default | back to the original value |
| apply | make a change take effect |
| key / hotkey / shortcut | a keyboard key bound to an action |
| mouse / wheel / button | pointer input |
| gesture | a mouse or touch movement bound to an action |
| touch | a touchscreen's input |
| controller / gamepad | an Xbox controller |
| video / picture / image | the video's picture, a still image |
| frame | one picture of the video |
| frame rate / fps | frames per second |
| aspect ratio | the picture's width to height |
| crop | cut the picture's edges |
| zoom / pan | enlarge / move the picture |
| rotate / flip / mirror | turn / upside down / left-right |
| deinterlace | remove interlacing |
| brightness / contrast / saturation / hue / gamma | the color controls |
| sharpen / denoise | make sharper / remove noise |
| scaler / upscaling / resize | how the picture is enlarged or reduced |
| pixel shader | a GPU effect on the picture |
| renderer | the video output (OpenGL, Direct3D 11, Vulkan) |
| hardware decoding / decoder | decoding by the GPU / what decodes |
| HDR | high dynamic range |
| 3D / 360° video / anaglyph / side by side / top and bottom | stereo and panoramic modes |
| bars / letterbox / margin | black areas around the picture |
| frame capture / screenshot | saving the current frame as an image |
| motion smoothing | making motion smoother by creating frames |
| frame interpolation | creating frames between the original ones |
| engine | one of the smoothing methods (MVTools, RIFE...) |
| calibration | measuring what this computer can do |
| quality preset (light, medium, high) | the engine's settings |
| GPU / graphics card | the video card |
| processor / CPU | the main processor |
| power budget | the share of the card's power RVP may use |
| dropped frames | frames not shown in time |
| refresh rate | the display's rate (Hz) |
| audio / sound | the sound of a file |
| volume / mute | loudness / silence |
| master volume | the system's own volume |
| equalizer | the frequency bands' levels |
| normalizer | evening out the loudness |
| reverb | the echo effect (Freeverb) |
| noise reduction | removing noise from the sound |
| channel / speaker | a sound channel / its speaker |
| downmix | mixing many channels into fewer |
| surround / stereo | multichannel sound / two channels |
| output device | where the sound plays |
| sample rate / bitrate | samples per second / bits per second |
| audio delay / audio sync | moving the sound against the picture |
| pitch / time stretching | the tone / speed without changing pitch |
| audio recording | saving what RVP plays |
| second audio device | playing on two devices at once |
| pass-through | sending the encoded sound to a receiver |
| subtitles / subtitle track | text over the video |
| external / embedded subtitles | from a separate file / inside the video |
| font / outline / shadow | how subtitles and the OSD are drawn |
| subtitle position / subtitle delay | where and when they show |
| encoding | a text file's character set |
| subtitle browser / subtitle editor | the windows listing and editing subtitle lines |
| subtitles from audio / speech recognition | subtitles made from the speech (whisper.cpp) |
| model | whisper's speech model |
| language | of the interface, a track, the speech |
| remote control | controlling RVP from another device |
| token | the secret that remote programs give RVP |
| port / server | the network port / the program that listens |
| log | RVP's record of what happens |
| crash | RVP ending by an error |
| download / install | fetching / putting files in place |
| installer | the Windows setup program |
| accessibility | help for users with disabilities |
| text to speech (TTS) / screen reader | reading texts aloud / the program for blind users |
| screen saver | the desktop's screen saver |
| shut down / sleep / hibernate | the computer's power actions |
| timer | a time after which an action runs |
| visualization / spectrum | pictures that follow the sound |
| status bar / info | the bars and texts showing the file's details |
| codec / format | the coding / the file's kind |
| bit depth / 10-bit | bits per color |
| channel layout (5.1, 7.1) | the speakers' arrangement |
| Auto select (recommended) | the automatic choice |
