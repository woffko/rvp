# Goal L status: R2 and milestone M10 (PotPlayer parity: audio)

Date: 2026-10-17. Result: **done**. Precondition met: `goal-j-status.md`
records goal J as done. The owner reported no findings from the audio
audit's build, so nothing came before M8's Windows gap. The host session
comparing RVP with PotPlayer ran with the owner's permission (two runs);
RVP's effects, presets, speaker levels and normalizer blocks were then
fitted to what it measured (`docs/audits/audio.md`, "M10").

## Summary

| Part | Result | Where |
|------|--------|-------|
| M8's Windows gap | While video plays (not paused, not audio alone) RVP asks Windows to keep the display and the system on (`SetThreadExecutionState`), with Playback's "Prevent screen saver during playback" (on by default); Linux's idle inhibit follows the same rule | `shell/src/awake.rs`, `App::keeps_screen_on` |
| R2: Preferences | Every control of PotPlayer's 76 Preferences pages (1,183 rows), read-only from its DLL and `English.ini`; the host session dumped the audio pages' run-time lists and values | `docs/r1/preferences.tsv`, `tools/r1/potplayer_tables.py` |
| R2: command line | PotPlayer's 29 switches | `docs/r1/command-line.tsv` |
| Coverage | `docs/coverage/preferences.md` (1,040 options: every audio page's option is a setting or has a reason, the rest planned by milestone); the menus' coverage counts the 720 commands of the live menu that the static tree lacks (4 carried out, 116 in lists RVP fills, 600 not); all of the Audio menu is carried out | `crates/app/src/coverage.rs`, `crates/app/src/coverage/preferences.rs` |
| Output | The Audio Renderer menus list mpv's devices (main, pass-through, 2nd); exclusive mode; Output Mode (PCM, pass-through of AC3, E-AC3, DTS, DTS-HD and TrueHD, from a channel count, or AC3 re-encoding at PotPlayer's bitrates); Output Format, sample rate and resampling quality; resampling always or only for sources below or above a rate | `crates/app/src/audio/mod.rs`, `audio/commands.rs`, `menu.rs` |
| Speakers | PotPlayer's 26 choices: the layouts 1.0 to 7.1 with and without LFE, Dolby Surround and Pro Logic II (swresample's matrix encodings without the LFE, as PotPlayer; "+LFE" keeps it on a channel), Same as input, the virtual headphone and surround (a spherical head model for FFmpeg's `headphone`, at PotPlayer's levels), the Dolby decoder (`surround`); the mixer's levels, centre and surround expansion, LFE redirection | `crates/app/src/speakers.rs`, `audio/chain.rs`, `audio/dsp.rs` |
| Second device | A second core plays the file to "2nd Audio Renderer" and follows the main one: file, pause, speed, track, volume, mute, seeks (ahead by its measured seek lag); the drift between the devices' clocks is corrected by small speed changes, beyond 0.25 s by a seek; a delay per device; files and discs only. The normalizer's gain comes from the main core. On the host it stayed within 0.6 ms of the first device | `crates/app/src/audio/second.rs` |
| Channels | Both, Left or Right channel of a stereo pair, Swap Stereo Channels | `@rvpchannel` |
| Volume | System Master volume (the default device's volume, Core Audio) and System Wave volume (mpv's `ao-volume`) as volume methods and in the System Volume menu; individual volume (a master and a gain per channel); AC3/DTS preamp with PreAmp up and down; "Adjust wave volume to max", accelerated keys, a startup volume | `shell/src/system_volume.rs`, `@rvpgains`, `@rvppreamp` |
| Equalizer | PotPlayer's 19 presets as measured on the host (its responses at the ten bands, within 0.8 dB), sliders of ±20 dB as PotPlayer's; SuperEQ as FFmpeg's `superequalizer` | `crates/app/src/equalizer.rs` |
| Sync | The Adjust Audio Sync dialog; keeping sync changes never, over the runtime or always; the default sync | `shell/qml/AudioSync.qml`, `audio/commands.rs` |
| Effects | One chain in one order (downmix, channel, preamp, equalizer, voice, Crystality, noise reduction, Freeverb, crossfeed, the user's filters, normalizer, channel gains, tempo, AC3), each fitted to PotPlayer's on the host: Voice Emphasis a band-pass at 1 kHz; Noise Reduction a low-pass at 1875 Hz; Crossfeed bs2b's filters at PotPlayer's 700 Hz and 6 dB; Freeverb Jezar's tuning unscaled; Crystality a rectifier, two shelves, extra stereo and its measured four-tap echo (a response RVP computes); Voice Removal the centre cancelled | `crates/app/src/audio/chain.rs`, `audio/dsp.rs` |
| Time stretching | High quality (`scaletempo2`), default quality (`scaletempo`), or pitch with the speed; Audio Stretching (Shift+H, J, K, 1 % a step): the pitch at the same speed, as PotPlayer's, with rubberband | `audio/chain.rs`, `audio/commands.rs` |
| Normalizer | Its block per source as measured: AC3 and E-AC3 1536, MP3 2304, Opus 2880, DTS 2048, PCM 3000, else 2048; "Operate only with video playback" | `crates/app/src/normalizer.rs` |
| Recording | Audio Recording: RVP's own sound (Windows' process loopback) as 32-bit float WAV, to a folder or beside the media | `shell/src/recorder.rs` |
| Tracks | The selection method (specified languages, specified stream, highest quality, the file's default), audio languages, a preferred stream, subtitles off for chosen languages, the track remembered per file | `audio/commands.rs`, `resume.rs` |
| Settings | Pages Audio, Volume, Equalizer, Normalizer, Effects and Sync with every option above (sliders for numbers) | `shell/qml/Settings.qml` |

## The host comparison

Two runs of `host-session m10` (each announced and allowed by the owner;
a third, earlier attempt was stopped and its state restored) recorded
PotPlayer's output on the audit's tones and `make_m10.py`'s passes; the
tables and method are in `docs/audits/audio.md`, "M10". PotPlayer's
registry key and playlists were saved before each run and written back
after it; a diff showed them unchanged. Its findings, as RVP now has
them:

- **Presets**: PotPlayer's are its own (±20 dB sliders, broad bands), not
  Winamp's values; RVP's presets are the measured responses, within
  0.8 dB.
- **Effects**: every level within about 1 dB of PotPlayer's (the noise
  0.6 dB apart in every pass, the recording's bandwidth). Crossfeed is
  bs2b, Freeverb runs its tuning unscaled, Voice Emphasis is a band-pass
  and Noise Reduction a low-pass; Crystality was taken apart: a magnitude
  booster, shelves, extra stereo and a four-tap echo of the sides.
- **Audio Stretching** is a pitch shift at the same speed (110 %: 1 kHz to
  1.1 kHz, same timing). RVP has it now.
- **Speakers**: 2.0 Stereo matches exactly; the Dolby modes drop the LFE;
  the virtual modes are about 13.5 dB below 2.0 Stereo, the LFE 4.6 to
  7 dB.
- **Normalizer blocks**: whole decoder frames (MP3 2304, Opus 2880, AC3
  and E-AC3 1536, DTS 2048, FLAC two or three frames); PCM a 16th of a
  second; AAC no single block.
- **Second device**: RVP's stayed within 0.6 ms of the first device over
  34 s (drift -0.2 ms a minute), measured on one capture clock.
- **Preferences**: the run-time lists and defaults (the AC3 bitrate list,
  the resampling condition, the selection methods, the normalizer's
  methods, Crossfeed's 700 Hz and 6 dB, Noise Reduction's threshold).

## Tests

- `tests/audio_chain.rs` (headless libmpv, `ao=pcm`): levels on synthetic
  tones for each stage: left/right channel and swap, voice removal,
  voice emphasis and noise reduction against PotPlayer's measured band
  levels, crossfeed against bs2b's, Crystality's echo (a side tone's
  louder than a centre tone's), the AC3/DTS preamp (+6 dB), Freeverb's
  tail, the virtual headphone (near ear louder, at PotPlayer's level),
  Pro Logic II, stereo spread and the LFE redirection, individual volume,
  every effect plays (Crystality with and without its echo), Audio
  Stretching (1100 Hz from 1 kHz, the same length), and the 19 presets'
  band gains.
- `tests/audio_toggles.rs`: every effect switched on and off while a file
  plays, Audio Stretching up, up and off (rubberband in and out, the
  pitch changed in the running filter), and a switch at the very end of
  a file (the change comes with the next file).
- `tests/second_device.rs`: the second core follows the load, a seek
  (within 60 ms), pause, speed and a 400 ms delay, and stops.
- Unit tests: the chain's order and graphs, the presets, the head model,
  Freeverb's and Crystality's responses, the resampling rule, the
  highest-quality track, the normalizer's blocks, the coverage tables.
- `tests/coverage.rs`: PotPlayer's 28 audio keys each trigger their action
  by default or their menu item gives a reason (Shift+X, DirectShow's
  audio transform filter, is the one left).
- `rvp --selftest`: Crystality on and off with Shift+C, the 26 speaker
  choices, the individual volume after the normalizer, Freeverb's
  response, the sync dialog, the second device, recording, the master
  volume, and Audio Stretching with Shift+J and Shift+K.
- `tools/audio-audit/m10.py` renders RVP's chain on the session's passes
  and compares it with the recordings (they stay out of the repository).

## Checks on the Windows VM

`tools/windows-vm/m10-all.ps1` on this commit's code, all pass: the
self-test (failures 0, exit 0, with the Audio Stretching steps), the awake
request listed while video plays and gone after it (session 0 lists it
under SYSTEM), the recording (2.1 s, a valid RIFF file; the VM has no
sound device, so silence), the master volume answering "element not
found" without a device, and `tests/audio_chain.rs` (15 tests) and
`tests/speakers.rs` on Windows' own FFmpeg (the four-channel echo
response, `aeval` and rubberband included). `C:\Users\<user>\rvp-test\rvp-l`
holds the portable build of 644c138 (file copies only; `rvp.exe` SHA-256
338e3e6e...).

GitHub CI started no jobs for a while (the account's billing, which the
owner sorted out). Its first run after that found a gap the other checks
could not: the Linux kit builds libmpv from source, and without
librubberband, so the packages' self-test failed at Audio Stretching on
Ubuntu 26.10 (finding 7). With 4842430 CI passes on Linux and Windows
(run 39030252051), the packages' self-tests (87 checks each, the .deb
and the AppImage) included. The change after 644c138 only adds an OSD
message for a libmpv without rubberband, which Windows' has.

## Findings

1. **Recording corrupted the heap on Windows.** `ActivateAudioInterfaceAsync`
   frees the activation parameters' blob it is given; Microsoft's sample
   passes one on the stack. RVP died with 0xC0000374, no crash log. The
   blob now comes from `CoTaskMemAlloc`.
2. **FFmpeg versions differ.** Linux's 6.1 has no `irnorm` on `afir`,
   Windows' newer FFmpeg deprecates `gtype`. The responses are written so
   that both versions' own normalization leaves them alone, and their
   levels are put back after the convolution. FFmpeg's `headphone` lowers
   its output by 3 dB per input channel and passes the LFE by its own
   `lfe` gain, not a response.
3. **mpv refuses `audio-format=no`** (the documented "auto"): RVP sets only
   the options that differ from mpv's defaults.
4. **PotPlayer applies a setting 0.7 to 1 s after its command**, and its
   menu's check marks follow its state only when a menu opens: the session
   read the state from the Preferences pages, and measured after the
   switch.
5. **Session 0 has no display**: Windows lists a display request there
   under SYSTEM only.
6. **The self-test misread Linux's refusal to record** once in CI; the
   status now says whether the recorder runs.
7. **The Linux kit's libmpv had no rubberband**: `kit.sh` builds mpv from
   source and its packages lacked `librubberband-dev`, so mpv refused the
   chain with Audio Stretching (Windows' libmpv and Ubuntu's have it). The
   kit now installs it and builds with `-Drubberband=enabled`; `MPV_BUILD`
   rebuilds a cached libmpv of the same commit. RVP says on the OSD when
   its libmpv has no rubberband.

## Differences from PotPlayer

- Noise Reduction is PotPlayer's at its default threshold; the threshold
  (1 to 5000) was not measured, so RVP has none. The normalizer's two
  other methods ("32-bit simple" and "complex arithmetic") were not
  measured; RVP runs TomSteady, the recommended one.
- Crystality and the echo were measured at their defaults; RVP's sliders
  scale its parts (the echo's repeats with the volume, the later ones with
  the feedback, the extender's lower shelf an octave per HPF step), which
  PotPlayer's may do differently. PotPlayer's echo wavers slightly in
  pitch, RVP's does not.
- Freeverb's and Crystality's responses are at 48 kHz: for a 44.1 kHz
  source RVP's Freeverb echoes come 8 % sooner than PotPlayer's probably
  do.
- The virtual modes use RVP's head model at PotPlayer's levels; PotPlayer's
  virtual headphone separates the ears less (1.7 dB at 1 kHz for a side
  speaker, RVP 5 dB).
- PotPlayer's Dolby modes and Virtual Surround dropped a 5.1 (side) WAV's
  surrounds; RVP encodes them.
- The normalizer's block for AAC (no single block fits) and for FLAC files
  whose frames are not 1024 samples (RVP cannot see the frame size) stays
  2048.
- Output: PotPlayer's "LPCM" has no separate choice in RVP (its PCM is
  linear PCM); RVP's pass-through choice is PotPlayer's pass-through
  settings.
- Winamp DSP plugins (PotPlayer's page) are not planned: they are Windows
  DLLs loaded into PotPlayer.

## Linux items for goal S

- The system's master volume through PipeWire or PulseAudio (Windows only
  now: "on Windows for now").
- Recording RVP's own PipeWire stream (Windows only now).
- The second device with Linux's devices (the code is mpv's and runs in
  CI on Linux with `ao=null`; real devices untested).
- PipeWire's own way to combine two outputs, as an alternative.

## What to try

In `C:\Users\<user>\rvp-test\rvp-l` (portable: its settings stay beside
`rvp.exe`), with PotPlayer's menu texts and keys:

1. **Equalizer presets** (Audio > Equalizer Presets; Shift+E switches the
   equalizer): PotPlayer's 19, now as PotPlayer plays them; compare one or
   two with PotPlayer on music.
2. **Effects** with PotPlayer's keys: Shift+C Crystality (its echo is the
   new part), Shift+D Noise Reduction, Shift+R Freeverb, Shift+V Voice
   Removal, Shift+A Voice Emphasis (now a voice band, as PotPlayer's),
   Shift+T Swap Stereo Channels; Crossfeed in Audio > Audio Effects (bs2b,
   for headphones).
3. **Audio Stretching**: Shift+J raises the pitch 1 % a step, Shift+H
   lowers it, Shift+K resets it; the speed stays.
4. **Output Speakers** (context menu Audio > Output Speakers, or Settings >
   Audio): 2.0 Stereo is PotPlayer's mix. With headphones, Virtual
   Headphone and Virtual Surround on a 5.1 film (quieter, as PotPlayer's);
   Dolby Pro Logic II on a 5.1 film through a receiver that decodes it.
5. **Second device**: with the monitor's audio enabled, Audio > Audio
   Renderer (2nd Device) plays the same sound there too; the settings page
   Audio has its delay for the display's own latency.
6. **Settings > Audio: Language/Sync/Other**: the selection method
   ("Prefer highest quality ones" picks the track with the most channels);
   Settings > Audio: the sample rate only for sources below or above a
   rate, and the AC3 re-encoding bitrate.
7. **Audio sync**, **recording** (Shift+G), **system volume**
   (Ctrl+Alt+Shift+Up and Down) and **screen saver** (the display stays on
   while video plays) as before.

## Open

- PotPlayer's Noise Reduction threshold, Crystality's sliders away from
  their defaults and the normalizer's other methods could be measured in a
  later host session with the same tools (`make_m10.py`, `host-session
  m10`, `m10.py`) if the owner wants them.
