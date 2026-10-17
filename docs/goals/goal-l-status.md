# Goal L status: R2 and milestone M10 (PotPlayer parity: audio)

Date: 2026-10-17. Result: **in progress**: everything but the host
comparison with PotPlayer is done, tested and pushed; the comparison
session waits for the owner's permission. Precondition met:
`goal-j-status.md` records goal J as done. The owner reported no findings
from the audio audit's build, so nothing came before M8's Windows gap.

## Summary

| Part | Result | Where |
|------|--------|-------|
| M8's Windows gap | While video plays (not paused, not audio alone) RVP asks Windows to keep the display and the system on (`SetThreadExecutionState`), with Playback's "Prevent screen saver during playback" (on by default); Linux's idle inhibit follows the same rule | `shell/src/awake.rs`, `App::keeps_screen_on` |
| R2: Preferences | Every control of PotPlayer's 76 Preferences pages (1,183 rows), read-only from its DLL and `English.ini` | `docs/r1/preferences.tsv`, `tools/r1/potplayer_tables.py` |
| R2: command line | PotPlayer's 29 switches | `docs/r1/command-line.tsv` |
| Coverage | `docs/coverage/preferences.md` (1,040 options: the audio pages' as settings or with a reason, the rest planned by milestone); the menus' coverage counts the 720 commands of the live menu that the static tree lacks (1 carried out, 116 in lists RVP fills, 603 not); all 52 commands of the Audio menu are carried out | `crates/app/src/coverage.rs`, `crates/app/src/coverage/preferences.rs` |
| Output | The Audio Renderer menus list mpv's devices (main, pass-through, 2nd); exclusive mode; Output Mode (PCM, pass-through of AC3, E-AC3, DTS, DTS-HD and TrueHD, from a channel count, or AC3 re-encoding); Output Format and sample rate; resampling quality | `crates/app/src/audio/mod.rs`, `audio/commands.rs`, `menu.rs` |
| Speakers | PotPlayer's 26 choices: the layouts 1.0 to 7.1 with and without LFE, Dolby Surround and Pro Logic II (swresample's matrix encodings, the LFE kept with "+LFE"), Same as input, the virtual headphone and surround (a spherical head model for FFmpeg's `headphone`), the Dolby decoder (`surround`); the mixer's levels, centre and surround expansion, LFE redirection | `crates/app/src/speakers.rs`, `audio/chain.rs`, `audio/dsp.rs` |
| Second device | A second core plays the file to "2nd Audio Renderer" and follows the main one: file, pause, speed, track, volume, mute, seeks (ahead by its measured seek lag); the drift between the devices' clocks is corrected by small speed changes, beyond 0.25 s by a seek; a delay per device; files and discs only. The normalizer's gain comes from the main core | `crates/app/src/audio/second.rs` |
| Channels | Both, Left or Right channel of a stereo pair, Swap Stereo Channels | `@rvpchannel` |
| Volume | System Master volume (the default device's volume, Core Audio) and System Wave volume (mpv's `ao-volume`) as volume methods and in the System Volume menu; individual volume (a master and a gain per channel); AC3/DTS preamp with PreAmp up and down; "Adjust wave volume to max", accelerated keys, a startup volume | `shell/src/system_volume.rs`, `@rvpgains`, `@rvppreamp` |
| Equalizer | PotPlayer's 19 presets (Winamp's values, clipped to ±12 dB, until PotPlayer's are measured); SuperEQ as FFmpeg's `superequalizer`; the filter is 20 ms long so that the steep low bands land within 1.5 dB | `crates/app/src/equalizer.rs` |
| Sync | The Adjust Audio Sync dialog; keeping sync changes never, over the runtime or always; the default sync | `shell/qml/AudioSync.qml`, `audio/commands.rs` |
| Effects | One chain in one order (downmix, channel, preamp, equalizer, voice, Crystality, noise reduction, Freeverb, crossfeed, the user's filters, normalizer, channel gains, tempo, AC3): Crystality from `aexciter`, `crystalizer`, `aecho`, `extrastereo`; noise reduction (`afftdn`); Freeverb (Jezar's algorithm run by RVP into an impulse response, convolved by `afir`); voice removal (centre cancel); voice emphasis (`dialoguenhance`'s centre); crossfeed | `crates/app/src/audio/chain.rs` |
| Time stretching | High quality (`scaletempo2`), default quality (`scaletempo`), or pitch with the speed | `audio/chain.rs` |
| Normalizer | Its block per source (AC3 and E-AC3 1536, PCM 3072, else 2048, as the audit measured), "Operate only with video playback" | `crates/app/src/normalizer.rs` |
| Recording | Audio Recording: RVP's own sound (Windows' process loopback) as 32-bit float WAV, to a folder or beside the media | `shell/src/recorder.rs` |
| Tracks | Audio language preference, a preferred stream, subtitles off for chosen languages, the track remembered per file | `audio/commands.rs`, `resume.rs` |
| Settings | Pages Audio, Volume, Equalizer, Normalizer, Effects and Sync with every option above (sliders for numbers) | `shell/qml/Settings.qml` |

## Tests

- `tests/audio_chain.rs` (headless libmpv, `ao=pcm`): levels on synthetic
  tones for each stage: left/right channel and swap, voice removal (the
  centre cancels, the sides stay), voice emphasis (the centre rises, a side
  does not), the AC3/DTS preamp (+6 dB), Freeverb's tail, the virtual
  headphone (near ear louder by more than 6 dB, at the speaker's level),
  Pro Logic II, stereo spread to the centre and the LFE redirection,
  individual volume, every effect plays, and the 19 presets' band gains
  (within 1.5 dB below 300 Hz, 1 dB above).
- `tests/audio_toggles.rs`: every effect switched on and off while a file
  plays (its label in the chain, playback on), and switched at the very
  end of a file (the album moves on, the change comes with the next file).
- `tests/second_device.rs`: the second core follows the load, a seek
  (within 60 ms), pause, speed and a 400 ms delay, and stops.
- Unit tests: the chain's order and graphs, the presets, the head model and
  Freeverb's response, the WAV writer of recordings, the coverage tables.
- `tests/coverage.rs`: PotPlayer's 28 audio keys (R1, the main window's
  table) each trigger their action by default, or their menu item gives a
  reason: 24 work; Shift+X (DirectShow's audio transform filter) and
  Shift+H, J, K (Audio Stretching, until the host session) have reasons.
- `rvp --selftest`: Crystality on and off with Shift+C, the 26 speaker
  choices, the individual volume in the chain after the normalizer,
  Freeverb's response, the sync dialog, the second device (offset 2.8 ms
  in WSL, 20 ms on the VM), recording and the master volume.

## Checks on the Windows VM

`tools/windows-vm/m10-all.ps1` on the code of 8113d23, the build in
`C:\Users\<user>\rvp-test\rvp-l` (`rvp.exe` SHA-256 af051329...), all
pass: the self-test (failures 0, exit 0), the awake request listed while
video plays and gone after it (session 0 lists it under SYSTEM; DISPLAY
needs a desktop session), the recording (2.2 s, a valid RIFF file; the VM
has no sound device, so silence), the master volume answering "element
not found" without a device, and `tests/audio_chain.rs` and
`tests/speakers.rs` on Windows' own FFmpeg. CI runs every test on Linux and
Windows.

## Findings

1. **Recording corrupted the heap on Windows.** `ActivateAudioInterfaceAsync`
   frees the activation parameters' blob it is given; Microsoft's sample
   passes one on the stack. RVP died with 0xC0000374, no crash log. The
   blob now comes from `CoTaskMemAlloc` (found on the VM with a manual
   test, `recorder::tests::records_its_own_sound`).
2. **FFmpeg versions differ.** Linux's 6.1 has no `irnorm` on `afir`,
   Windows' newer FFmpeg deprecates `gtype`. Freeverb's response is
   written so that both versions' own normalization leaves it alone, and
   the level is put back after the convolution. FFmpeg's `headphone`
   lowers its output by 3 dB per input channel: the gain gives it back.
3. **mpv refuses `audio-format=no`** (the documented "auto"): RVP sets only
   the options that differ from mpv's defaults, and a change back to
   "Same as input" applies at the next start.
4. **The equalizer's 10 ms filter missed steep bands**: Full Bass's 60 Hz
   came out 3 dB off. 20 ms brings every preset within 1.5 dB.
5. **Session 0 has no display**: Windows lists a display request there
   under SYSTEM only.
6. **The self-test misread Linux's refusal to record** ("Audio recording:
   on Windows for now" starts like a started recording's OSD) when the
   text stayed on screen two seconds: CI's .deb check on Ubuntu 26.10
   failed once. The status now says whether the recorder runs.

## Differences from PotPlayer (before the host comparison)

- Crystality, noise reduction and crossfeed are built from FFmpeg's filters
  with PotPlayer's parameters mapped onto them; Freeverb is Jezar's
  algorithm, PotPlayer's own code is unknown. The comparison will fit them.
- The virtual surround uses RVP's head model, not an HRTF set (FFmpeg's
  `sofalizer` needs libmysofa, which neither build has).
- The presets are Winamp's values (in VLC's dB form) until PotPlayer's are
  measured. A read-only search of `PotPlayer64.dll` for a preset table (dB
  as floats, doubles or tenths, Winamp's 0-63 slider values, and rows
  shaped like (Default), Classical, Club and Dance) found none: the DLL
  holds the names only, in its string table. The host session measures
  them.
- Winamp DSP plugins (PotPlayer's page) are not planned: they are Windows
  DLLs loaded into PotPlayer.

## Linux items for goal S

- The system's master volume through PipeWire or PulseAudio (Windows only
  now: "on Windows for now").
- Recording RVP's own PipeWire stream (Windows only now).
- The second device with Linux's devices (the code is mpv's and runs in
  CI on Linux with `ao=null`; real devices untested).
- PipeWire's own way to combine two outputs, as an alternative.

## Open

- The host session comparing RVP with PotPlayer: presets, effects,
  speaker modes, normalizer blocks for MP3, AAC, Opus, PCM, AC3, E-AC3,
  DTS and 44.1 kHz FLAC, and PotPlayer's audio Preferences pages read
  (combo lists, slider ranges, defaults, Audio Stretching's meaning).
  Tools ready: `tools/audio-audit/make_m10.py`, `host-session m10`,
  `tools/audio-audit/m10.py`.
- The second device on the host needs a second active output: only "SPDIF
  Interface" is active now (the monitor's HDMI audio is not present).
- `C:\Users\<user>\rvp-test\rvp-l` holds the portable build of 8113d23
  (file copies only); it is refreshed if the comparison changes code.
