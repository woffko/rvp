# Audio audit against PotPlayer (2026-10-16)

The owner compared RVP with PotPlayer by ear and asked for a full audit
(2026-10-16):

- "PotPlayer's 20 % sounds like RVP's 40 %. A TomSteady shortcoming?"
- The volume maximum set to 100 % came back as 200 %.
- The volume slider stops at 100 % even with a higher maximum, and shows no
  OSD (the keys do).
- Choosing an item in the context menu also paused the video.
- No audio settings: neither 5.1 nor stereo; PotPlayer has them, "and the
  other settings went missing too".
- Sliders are preferred to typed numbers in the settings.

This file records what was measured, what differed, what changed, and what
RVP still lacks.

## Method

One session on the owner's host, with permission (2026-10-16, about 4
minutes): `spikes/host-session` `audio` recorded the default output
device (FiiO Q series over S/PDIF, 44.1 kHz) with WASAPI loopback while:

1. RVP (the portable build of goal J) played `tone-stereo.flac` at volume
   100 with its normalizer on and off, and at volume 50 and 22 with it off;
   then `tone-51.wav` and `tone-51-ac3.mkv` with the normalizer on;
2. PotPlayer, set up as the owner has it (normalizer on: TomSteady 32-bit
   float, Boost on, Level 75 %, Release time 10; Output Speakers at its
   default, 2.0 Stereo), played the stereo tone while the tool set its
   volume from 100 % down to 0 % and then 150 % and 200 % (2.5 s each,
   with PotPlayer's remote messages `WM_USER` `0x5001`, read back with
   `0x5000`), and then the two 5.1 files at 100 %.

The tones come from `tools/audio-audit/make-tones.sh`: a 1 kHz tone at
0.95 of full scale (above TomSteady's target, so both normalizers settle by
cutting and every block has the same peak), and a 5.1 file with one channel
at a time. `tools/audio-audit/audit.py` measures each part of the recording;
`tools/audio-audit/fit_downmix.py` fits PotPlayer's 5.1 mix with the S5
TomSteady model. The recording, PotPlayer's state before and after, and the
tones stay in `C:\Users\<user>\rvp-test\audio-audit` (PotPlayer's registry
values and playlist were written back afterwards; only `MInfo1`, `MInfo2`
and the root `(Default)`, which PotPlayer rewrites on every start, differ).

Loopback gives no samples while nothing plays, so the parts were found by
their levels, not by the log's times. RVP's tone at volume 100 without the
normalizer recorded at exactly the tone's level: the capture added 0.00 dB.

## Findings

### 1. Volume curve: the cause of "20 % sounds like 40 %"

| Volume | PotPlayer, dB | Square law (40 · log10) | mpv's cube (RVP before) |
|-------:|--------------:|------------------------:|------------------------:|
| 80 % | -3.88 | -3.88 | -5.81 |
| 70 % | -6.18 | -6.20 | -9.29 |
| 60 % | -8.88 | -8.87 | -13.31 |
| 50 % | -11.98 | -12.04 | -18.06 |
| 40 % | -15.88 | -15.92 | -23.88 |
| 30 % | -20.88 | -20.92 | -31.37 |
| 25 % | -24.08 | -24.08 | -36.12 |
| 22 % | -26.28 | -26.30 | -39.45 |
| 20 % | -27.98 | -27.96 | -41.94 |
| 15 % | -32.98 | -32.96 | -49.43 |
| 10 % | -39.99 | -40.00 | -60.00 |
| 7 % | -46.20 | -46.20 | -69.29 |
| 5 % | -52.01 | -52.04 | -78.06 |
| 3 % | -60.99 | -60.92 | -91.37 |
| 2 % | -68.11 | -67.96 | -101.94 |
| 1 % | -80.84 | -80.00 | -120.00 |

PotPlayer's levels are relative to its settled level at 100 % (100 % and
90 % themselves were measured while its normalizer still settled from the
start). PotPlayer scales the sound by `(volume / 100)²`; mpv by
`(volume / 100)³`. At the owner's 22 % RVP was 13 dB quieter than PotPlayer.

Above 100 % PotPlayer accepts 150 % and 200 % and amplifies linearly, then
clips at full scale: the clipped tone measured -2.37 dB RMS with a crest
factor of 2.38 dB at 150 % and -1.54 dB with 1.59 dB at 200 %; a linear
gain of 1.5 and 2 followed by clipping predicts -2.32/2.32 dB and
-1.51/1.51 dB.

**Changed:** RVP shows and takes PotPlayer's percent everywhere (keys,
wheel, sliders, OSD, the highest volume, MPRIS) and gives mpv the volume of
the same gain (`crates/app/src/volume.rs`: square law up to 100 %, linear
above). The state file keeps mpv's volume, so a saved volume keeps its
loudness and shows PotPlayer's number for it: the owner's saved 41 now
shows as 26 %.

### 2. Normalizer (TomSteady): matches

| | Settled level of the tone at 100 % |
|---|---|
| PotPlayer | -5.43 dB (from its levels at 3-80 %) |
| RVP | -5.51 dB (TomSteady's 75 % target) |

The 0.08 dB between them is within the resampling to 44.1 kHz. Both cut the
tone to TomSteady's ceiling at once and then lower the gain at the same
rate (about 0.21 dB/s on the stereo tone). The normalizer was not the cause.

One difference remains: PotPlayer runs TomSteady over the blocks its audio
pipeline hands it, which depend on the source. The fits give 2048 samples
for stereo FLAC (S5), 3072 for the 5.1 WAV and 1536 (an AC3 frame) for the
AC3 file; RVP uses 2048 for all. This changes only how fast the gain moves
(an AC3 film's gain moves a third faster in PotPlayer), not the level. Not
changed; it needs measurements of the codecs the owner's films use (AC3,
E-AC3, AAC, DTS) before a rule can be written.

### 3. 5.1 sources: PotPlayer mixes down first

PotPlayer's default Output Speakers is "2/0/0 - 2.0 Stereo (Recommended)":
every source becomes stereo before its normalizer. The fit of TomSteady on
the mix against the recording:

| File | Block | Centre | LFE | Surround | RMS error |
|---|---:|---:|---:|---:|---:|
| tone-51.wav | 3072 | -3.0 dB | -3.0 dB | +0.1 dB | 0.015 dB |
| tone-51-ac3.mkv | 1536 | -3.1 dB | -3.1 dB | 0.0 dB | 0.074 dB |

Front channels at their level, centre and LFE at -3 dB, surround channels
at their level, the sum not scaled down. Normalizing before the mix fits
far worse (0.87 and 1.04 dB), as does mpv's matrix (1.56 and 1.20 dB).
PotPlayer ignores the AC3 stream's own downmix levels (-4.5 dB centre, -6 dB
surround in these files), and adds no AC3 pre-amplification (the AC3 file's
level matched the model within 0.1 dB).

RVP before: the normalizer measured all channels before mpv mixed them down
with the surround channels at -3 dB and without the LFE. A film's dialogue
(centre) came out about 3 dB quieter than PotPlayer's, the surround
channels 3 dB quieter, and the LFE was missing.

**Changed:** a speakers setting (`output-speakers`, PotPlayer's Output
Speakers, `crates/app/src/speakers.rs`): Stereo, the default, puts
PotPlayer's mix in front of RVP's chain (`@rvpdownmix`, swresample with the
levels above), so the equalizer and the normalizer work on the stereo mix as
in PotPlayer; Mono, 2.1, 5.1, 7.1 and Same as input leave the channels to
mpv. In the settings window (Audio, Output) and in the context menu (Audio,
Output Speakers, with PotPlayer's texts). `tests/speakers.rs` checks the
levels on a synthetic 5.1 file (also on the Windows VM and with the kit's
FFmpeg).

### 4. Settings window: typed numbers never applied

The settings window defined its own component named `Number`, which hid
JavaScript's `Number()` in that file: `Number(text)` threw, so no typed
number (seek steps, volume step and maximum, normalizer level and release,
sync steps, colours, buffered frames, subtitle size) was ever applied; the
field showed the typed text until the page reloaded. The self-test set
values through the window's `set()` and missed it.

**Changed:** every number is a slider with its value beside it (the owner
prefers sliders); stepped sliders over sensible values for the seek and
sync steps and the maximum. The self-test now moves the sliders
(`settings_sliders_apply`).

### 5. Volume slider: range and OSD

The skin's and the control panel's volume sliders ran from 0 to 100 % and
changed the volume silently. **Changed:** they run from 0 to the highest
volume and show "Volume: N %" like the keys (self-test
`volume_slider_reaches_max`). mpv's `volume-max` is now set a thousandth
above the highest volume: with three decimals it fell below it, and mpv
refused the highest volume itself (`tests/volume.rs`).

### 6. Context menu: the click went through

A press outside an open menu (on the parent menu while a submenu was open,
or on the video) closes the menu in Qt and goes on to the window beneath;
on the video it was a click, and play/pause toggled. PotPlayer's native
menus keep such a click. **Changed:** the main window and the control panel
ignore a press that comes while a menu is open or within 250 ms after one
began to close, and its release (self-test `menu_closing_press_kept`,
`menu_closing_press_no_pause`).

## What RVP lacks

(As of this audit. Goal L added most of it; its comparison with PotPlayer
is in "M10" below, and `docs/coverage/` lists what remains with reasons.)

PotPlayer's Audio menu (R1) against RVP after this audit:

| PotPlayer | RVP | How, with mpv |
|---|---|---|
| Select Audio Stream, Cycle | yes | |
| Both / Left / Right channel | no | `af pan` |
| Audio Renderer, (Pass-through), (2nd Device) | device in Settings only | menu over `audio-device-list`; a second device needs a second output (large) |
| Output Speakers | Mono, 2.0 (PotPlayer's mix), 2.1, 5.1, 7.1, Same as input | 3.0-6.1 layouts, Dolby Surround/Pro Logic (swresample's matrix encoding), virtual headphone/surround (`sofalizer` with an HRTF file) |
| Output Format/Resample (bits, rate, quality) | no | `audio-format`, `audio-samplerate`, `audio-swresample-o` |
| Output Mode (PCM, LPCM, pass-through) | no | `audio-spdif` (needs a receiver to test) |
| Playback Volume: default control | yes, PotPlayer's curve | |
| Playback Volume: system wave/master control, individual volume | no | the system's volume APIs (WASAPI, PipeWire) |
| Pre-amplify AC3/DTS, PreAmp up/down | no | a gain stage (`volume-gain` or `af volume`) |
| System Volume (master/wave up, down, mute) | no | the system's volume APIs |
| Increase/Decrease Volume, Mute | yes | |
| Use Equalizer | yes | |
| Equalizer Presets | RVP's own 13 | PotPlayer's 19 Winamp presets (Classical, Club, Dance, Full Bass, Full Bass & Treble, Full Treble, Laptop, Large Hall, Live, Loudness, Party, Pop, Reggae, Rock, Ska, Soft, Soft Rock, Techno) need their values |
| Audio Sync: steps, default | yes | |
| Audio Sync: Adjust..., keep changes (never, whole runtime, always) | no | a dialog; the resume store |
| Normalizer | yes, measured | |
| Crystality | no | `crystalizer` |
| Noise Reduction | no | `afftdn` |
| Freeverb | no | no ffmpeg reverb; `afir` with an impulse response |
| Voice Removal, Voice Emphasis | no | `pan` / `stereotools` centre cut; an equalizer band |
| Swap Stereo Channels | no | `af pan` |
| Crossfeed | no | `crossfeed` |
| Audio Recording | no | a recording tap after the chain |
| Built-in Audio Transform Filter | not applicable (DirectShow) | |

The rest of the context menu is in the same state: M4 limited it to what
RVP had. Commands per top-level menu, PotPlayer's (R1) and RVP's:

| Menu | PotPlayer | RVP |
|---|---:|---:|
| Open | 33 | 6 |
| Playback | 120 | 46 |
| Subtitles | 68 | 17 |
| Video | 185 | 42 |
| Audio | 53 | 30 (with RVP's lists) |
| Misc | 35 | 8 |
| Frame Size / Aspect Ratio / Window Size | 80 | 33 |
| DVD Control | 21 | 2 |
| Tools, Visualization, TV, Skins | 62 | 0 |

## Open

- A run on the host comparing RVP after this audit with PotPlayer on the
  same tones (and a film's 5.1 track) would confirm the match by ear and by
  level. (Goal L's session below compared the new functions.)
- The normalizer's block per source (finding 2): measured in M10 below.
- Which of the missing functions to add, and in what order (the owner's
  choice): goal L added the audio ones.

## M10: goal L's comparison with PotPlayer (2026-10-17)

### Method

Two sessions on the owner's host, with permission, recorded the default
device (FiiO Q series over S/PDIF, 44.1 kHz) with WASAPI loopback while
`spikes/host-session` `m10` played `tools/audio-audit/make_m10.py`'s files
in PotPlayer and switched its settings through its menu commands
(`WM_COMMAND`) in the silence before each pass (each pass starts with a
2 kHz marker on the left):

1. run 2: the 19 equalizer presets on ten tones (the bands' frequencies,
   -26 dBFS), then no effect, each effect and Audio Stretching at 110 % on
   a pass of tones, a noise burst and a noise floor; then RVP (goal L's
   build) playing pips to the default device and its second device (the
   monitor's HDMI audio), both recorded with their capture times;
2. run 3: 2.0 Stereo and five speaker modes on a 5.1 file with one channel
   at a time, then the normalizer (on in a silent instance first) on the
   audit's 1 kHz tone in eight codecs.

PotPlayer's state (the bits each switch depends on) was read from its
Preferences pages, which the tool also dumped (every control's text,
value and list; `docs/r1/preferences.tsv` gives the controls). Before each
run its registry key and playlist folder were saved and afterwards written
back; a registry diff and the playlists were the same as before. RVP's
side is rendered through RVP's own chain (`render_chain`) and measured the
same way by `tools/audio-audit/m10.py`, which also calibrates PotPlayer's
recording: it reached the recording 12.4 dB lower than RVP's renders (the
host's volume), measured on the left-only 1 kHz tone of the pass without
an effect, and FR of 2.0 Stereo.

PotPlayer applies a switch 0.7 to 1 s after the command: the first
tone of a pass still plays the previous setting. The measurements start
after that (the 170 Hz band from 0.1 s, the centre tone from 0.5 s); each
preset's 60 Hz is the next pass's (unknown for the last, Techno; RVP keeps
an estimate). Virtual Dolby Decoder restarts PotPlayer's output and lost
its pass's marker; the pass is found by its first sound.

### Equalizer presets

PotPlayer's presets are not Winamp's values: its sliders reach ±20 dB, and
its bands are broad and overlap, so a preset's response at each band is
not the slider. RVP's equalizer now takes ±20 dB and its presets are these
measured responses (gain at each band against (Default), dB); RVP renders
them within the last column:

| Preset | 60 Hz | 170 Hz | 310 Hz | 600 Hz | 1000 Hz | 3000 Hz | 6000 Hz | 12000 Hz | 14000 Hz | 16000 Hz | RVP within |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| (Default) | +0.0 | +0.0 | +0.0 | +0.0 | +0.0 | +0.0 | +0.0 | +0.0 | +0.0 | +0.0 | 0.0 dB |
| Classical | +0.0 | +0.0 | +0.0 | -0.1 | -0.3 | -2.5 | -7.3 | -8.3 | -8.1 | -7.6 | 0.0 dB |
| Club | -0.1 | +0.4 | +1.3 | +2.3 | +2.3 | +1.8 | +1.2 | +0.3 | +0.2 | +0.1 | 0.0 dB |
| Dance | +3.3 | +3.9 | +2.7 | +0.7 | -0.1 | -2.2 | -3.0 | -1.8 | -1.5 | -1.2 | 0.1 dB |
| Full Bass | +3.4 | +5.7 | +6.0 | +4.2 | +2.1 | -3.4 | -7.8 | -9.4 | -9.2 | -8.6 | 0.2 dB |
| Full Bass & Treble | +1.9 | +5.7 | +0.1 | -1.9 | -1.2 | +5.4 | +9.7 | +10.0 | +9.6 | +9.1 | 0.5 dB |
| Full Treble | -3.1 | -4.7 | -4.7 | -1.4 | +2.0 | +13.0 | +18.5 | +17.5 | +16.6 | +15.6 | 0.1 dB |
| Laptop | +1.3 | -4.7 | +2.9 | +0.0 | -0.0 | +4.3 | +7.7 | +8.7 | +8.6 | +8.3 | 0.8 dB |
| Large Hall | +3.4 | +4.9 | +4.4 | +2.6 | +1.0 | -2.2 | -3.3 | -2.4 | -2.1 | -1.7 | 0.1 dB |
| Live | -1.9 | +0.0 | +1.1 | +2.0 | +2.1 | +2.0 | +1.6 | +0.9 | +0.8 | +0.7 | 0.2 dB |
| Loudness | +6.1 | +7.8 | +6.9 | +4.3 | +3.1 | +7.4 | +11.8 | +13.0 | +12.8 | +12.4 | 0.2 dB |
| Party | +2.0 | +7.8 | +1.2 | +0.4 | +0.4 | +1.6 | +2.8 | +3.6 | +3.6 | +3.6 | 0.7 dB |
| Pop | -0.6 | +1.7 | +2.7 | +2.8 | +1.9 | -1.5 | -3.0 | -2.8 | -2.6 | -2.3 | 0.2 dB |
| Reggae | +0.0 | +1.7 | -0.9 | -1.8 | -0.5 | +2.6 | +2.6 | +0.9 | +0.5 | +0.3 | 0.2 dB |
| Rock | +2.7 | +1.6 | -1.5 | -3.1 | -1.7 | +5.1 | +8.4 | +8.2 | +7.8 | +7.3 | 0.1 dB |
| Ska | -0.7 | -1.6 | -1.1 | +0.1 | +1.1 | +4.2 | +6.4 | +6.2 | +5.9 | +5.4 | 0.1 dB |
| Soft | +0.9 | +0.3 | -0.4 | -0.9 | +0.0 | +5.1 | +8.5 | +8.8 | +8.5 | +8.1 | 0.0 dB |
| Soft Rock | +1.0 | +1.4 | +0.9 | -0.4 | -1.0 | -1.0 | +0.3 | +1.7 | +1.9 | +2.0 | 0.1 dB |
| Techno | — | +2.1 | +0.4 | -2.0 | -1.7 | +2.1 | +4.7 | +5.2 | +5.0 | +4.7 | 0.1 dB |

### Effects

Levels in dB of full scale, both players at gain 1 with their normalizers
off, PotPlayer calibrated as above (`left`: a 1 kHz tone on the left only,
its level on each side; `mix`: 500 Hz in the centre, 3 kHz left and 5 kHz
right, `mix_L5k` the 5 kHz that reached the left; `burst`: 0.2 s of white
noise; `tail`: what follows it, 0.05 to 0.3 s, 0.4 to 0.9 s and 1 to 1.7 s
after its end; `floor_rest`: a -46 dBFS noise floor under a 1 kHz tone,
the tone left out):

| Effect | Player | left_L | left_R | mix_L500 | mix_L3k | mix_R5k | mix_L5k | burst | tail_0.1 | tail_0.5 | tail_1.2 | floor_rest |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| none | PotPlayer | -15.1 | — | -23.0 | -23.0 | -23.0 | — | -19.4 | — | — | — | -46.6 |
|  | RVP | -15.1 | — | -23.0 | -23.0 | -23.0 | — | -18.8 | — | — | — | -46.0 |
| Crystality | PotPlayer | -13.5 | -33.9 | -22.4 | -21.5 | -21.5 | -41.2 | -16.9 | -44.6 | -48.5 | -66.2 | -40.7 |
|  | RVP | -13.8 | -34.3 | -22.6 | -21.3 | -21.2 | -40.6 | -16.0 | -44.5 | -49.0 | -64.3 | -39.9 |
| Noise Reduction | PotPlayer | -15.8 | — | -23.2 | -31.2 | -41.1 | — | -29.1 | — | — | — | -56.4 |
|  | RVP | -15.1 | — | -22.9 | -31.5 | -40.5 | — | -29.1 | — | — | — | -56.3 |
| Freeverb | PotPlayer | -10.3 | -27.4 | -24.0 | -19.1 | -18.6 | -31.1 | -15.6 | -32.4 | -56.5 | -88.6 | -33.8 |
|  | RVP | -10.3 | -27.4 | -24.0 | -19.1 | -18.6 | -31.1 | -15.0 | -32.0 | -56.5 | -89.4 | -33.7 |
| Voice Removal | PotPlayer | -15.1 | -15.1 | — | -23.0 | -23.0 | -23.0 | -16.3 | — | — | — | -43.6 |
|  | RVP | -15.1 | -15.1 | — | -23.0 | -23.0 | -23.0 | -15.8 | — | — | — | -43.0 |
| Voice Emphasis | PotPlayer | -15.1 | — | -35.9 | -40.9 | -46.1 | — | -35.7 | — | — | — | -62.6 |
|  | RVP | -15.1 | — | -36.0 | -41.0 | -46.2 | — | -35.4 | — | — | — | -62.6 |
| Crossfeed | PotPlayer | -17.5 | -29.4 | -23.6 | -24.8 | -24.7 | -49.5 | -21.1 | — | — | — | -48.3 |
|  | RVP | -17.5 | -29.4 | -23.6 | -24.8 | -24.7 | -49.5 | -20.5 | — | — | — | -47.7 |

The noise's levels differ by about 0.6 dB in every row, the pass without
an effect too: the 44.1 kHz recording leaves out the 48 kHz noise above
22 kHz. What each effect is, and what changed in RVP:

- **Voice Emphasis** keeps the voice's band: 1 kHz as it was, 500 Hz
  13 dB down, 3 kHz 18 dB, 5 kHz 23 dB. RVP's band-pass at 1 kHz (Q 2.9)
  replaced `dialoguenhance`, which raised the centre instead.
- **Noise Reduction** at its default threshold is a fixed low-pass:
  3 kHz 8 dB down, 5 kHz 18 dB, noise 10 dB. RVP's low-pass at 1875 Hz
  (Q 0.76) replaced `afftdn`; how PotPlayer's threshold moves it was not
  measured, so RVP has no threshold (`docs/coverage/preferences.md`).
- **Crossfeed** is bs2b: at PotPlayer's "Remove low frequencies" 700 Hz and
  "Gain" 6 dB (bs2b's Chu Moy setting) bs2b's own formulas give every
  level within 0.1 dB. RVP now builds bs2b's filters from FFmpeg's (FFmpeg's
  `crossfeed` filter is not bs2b) with PotPlayer's defaults.
- **Freeverb** runs Jezar's tuning in samples at the stream's rate, not
  scaled from 44.1 kHz: unscaled, RVP's response matches within 0.1 dB;
  scaled, it was up to 9 dB off. (PotPlayer probably runs a 44.1 kHz
  source at 44.1 kHz; RVP's response is at 48 kHz, where those echoes come
  8 % sooner.)
- **Voice Removal** cancels the centre in both (PotPlayer's centre tone
  -86 dB, RVP's -100 dB): no change.
- **Crystality** (its defaults are those of the XMMS Crystality plugin) was
  taken apart from the recording: the harmonic booster adds the signal's
  magnitude (even harmonics only, the second 30 dB under a tone at 43), the
  bandwidth extender raises everything 0.6 dB and noise above 16 kHz 3 dB
  more, extra stereo widens the sides (a side tone leaks 20 dB down,
  inverted), and the echo is four repeats of 17500, 28000, 35000 and
  45500 samples (at 48 kHz) with a gain from each side to each side,
  mostly the sides' difference (a centre tone's echo is 32 dB down, a side
  tone's 25 dB), each pass low-passed, and a weak tail to 2 s. RVP
  rebuilt it from that: `aeval`'s `|x|`, two high shelves, `extrastereo`
  and the echo as a computed response (`dsp::crystality_echo`, convolved
  by `afir`); before, its own mix of `aexciter`, `crystalizer` and `aecho`
  was up to 15 dB louder. Only the defaults were measured; the sliders
  scale the parts. PotPlayer's echo also wavers slightly (±4 to 10 Hz
  sidebands on a tone), RVP's does not.

**Audio Stretching** (Audio > Audio Stretching, Shift+H, J, K; 1 % a step,
"Audio Stretching : 101%" on the OSD) is a pitch shift at the same speed:
at 110 % the 1 kHz tone played at 1100 Hz and the pass kept its timing.
RVP now has it, with rubberband's `pitch-scale` in its tempo filter's place
(both libmpv builds have rubberband).

### Speaker modes

Left / right level of each channel of the 5.1 (side) file, dB. FL, the
first channel of a pass, still played the previous mode in PotPlayer:

| Mode | Player | FL | FR | FC | LFE | SL | SR |
|---|---|---|---|---|---|---|---|
| 2.0 Stereo | PotPlayer | -9.0 / — | — / -9.0 | -12.0 / -12.0 | -12.0 / -12.0 | -9.0 / — | — / -9.0 |
|  | RVP | -9.0 / — | — / -9.0 | -12.0 / -12.0 | -12.0 / -12.0 | -9.0 / — | — / -9.0 |
| Virtual Headphone | PotPlayer | (-13.7 / -37.4) | -25.5 / -23.8 | -27.1 / -27.1 | -16.6 / -16.6 | -26.3 / -28.0 | -28.0 / -26.3 |
|  | RVP | -24.3 / -27.5 | -27.5 / -24.3 | -26.3 / -26.3 | -17.8 / -17.8 | -22.2 / -27.5 | -27.5 / -22.2 |
| Virtual Surround | PotPlayer | (-20.1 / -27.6) | -24.1 / -16.3 | -22.6 / -22.6 | -19.0 / -19.0 | — / — | — / — |
|  | RVP | -21.1 / -24.3 | -24.2 / -21.1 | -23.1 / -23.1 | -17.8 / -17.8 | -18.9 / -24.3 | -24.2 / -19.0 |
| Dolby Surround | PotPlayer | (-16.2 / -26.0) | — / -9.0 | -12.0 / -12.0 | — / — | — / — | — / — |
|  | RVP | -9.0 / — | — / -9.0 | -12.0 / -12.0 | — / — | -12.0 / -12.0 | -12.0 / -12.0 |
| Dolby Pro Logic II | PotPlayer | -9.0 / — | — / -9.0 | -12.0 / -12.0 | — / — | — / — | — / — |
|  | RVP | -9.0 / — | — / -9.0 | -12.0 / -12.0 | — / — | -7.3 / -12.0 | -12.0 / -7.3 |

- 2.0 Stereo matches exactly.
- PotPlayer's virtual modes play a speaker about 13.5 dB below its 2.0
  Stereo and the LFE 4.6 to 7 dB below: RVP's virtual modes now have the
  same levels (`headphone`'s gain and its `lfe`). The ears' difference is
  RVP's head model's (PotPlayer's virtual headphone gives a side speaker
  only 1.7 dB more in the near ear at 1 kHz, RVP's 5 dB).
- PotPlayer's Dolby Surround and Pro Logic II leave the LFE out: RVP's
  now do too ("+LFE" keeps it on a channel of its own).
- PotPlayer's Dolby modes and Virtual Surround left the file's side
  surrounds out entirely. The file is a 5.1 (side) WAV (mask 0x60F); a
  decoder's 5.1 may name its surrounds otherwise, so this may be how
  PotPlayer reads this mask rather than its matrix. RVP encodes the
  surrounds, as the matrices are meant to.
- Virtual Dolby Decoder outputs 5.1 in both players; PotPlayer's went to
  the stereo device through Windows' own downmix, RVP's render is the 5.1
  itself, so their levels are not comparable here.

### Normalizer blocks

TomSteady's block that fits PotPlayer's gain steps on each file (in the
source's samples, 16-sample grid; the fit is about 1 % short on the
frame-sized blocks, so it reads 2288 for 2304), and RVP's rule:

| File | Codec | PotPlayer | fit RMS, dB | RVP |
|---|---|---:|---:|---:|
| block-mp3.mp3 | MP3 | 2288 | 0.005 | 2304 (2 frames) |
| block-aac.m4a | AAC | — (no single block fits) | 0.161 | 2048 |
| block-opus.opus | Opus | 2848 | 0.004 | 2880 (3 frames) |
| block-pcm16.wav | PCM | 2992 | 0.003 | 3000 (1/16 s) |
| block-ac3-stereo.mkv | AC3 | 1536 | 0.004 | 1536 (1 frame) |
| block-eac3.mkv | E-AC3 | 1520 | 0.006 | 1536 (1 frame) |
| block-dts.mkv | DTS | 2032 | 0.005 | 2048 (4 frames) |
| block-flac44.flac | FLAC, 44.1 kHz, 925-sample frames | 2768 | 0.003 | 2048 |

The blocks are whole decoder frames: the FLAC file's 925-sample frames
took three (2775), where S5's and the audit's FLAC (1024-sample frames)
took two (2048). RVP cannot see a FLAC file's frame size and keeps 2048.

### Second device

RVP played pips (1 kHz, 20 ms, every 0.5 s, 34 s) to the FiiO and, as
its second device, to the monitor's HDMI audio; both loopback recordings
carry their capture times (QPC), so each pip's arrival on the two devices
is compared on one clock. After the first pip (the second device started
30 ms early) the second device stayed within 0.6 ms of the first: mean
+0.06 ms, standard deviation 0.33 ms, drift -0.2 ms a minute. Loopback
sees the stream as Windows mixes it; each device's own output latency
(an HDMI display's in particular) comes on top, which the second device's
delay setting is for.

### PotPlayer's audio Preferences, read

The dump answered the lists PotPlayer fills at run time: output format
(16-bit (Recommended), 24-bit, 32-bit, 32-bit float, 16-bit with
dithering), sample rate (Same as input, 192000 to 8000 Hz), resampling
quality (High, Normal (Recommended), Low), resampling condition (Always,
when the source's rate is smaller or larger than an edit, 48000), output
(PCM, LPCM, Pass-through after AC3 re-encoding), the AC3 bitrate (64 to
640 kbps, 640 set), the normalizer's method (TomSteady 32-bit float
(Recommended), 32-bit simple, 32-bit complex) and the audio selection
method (Prefer specified languages (Recommended), Prefer specified
stream, Prefer highest quality ones, Use default selection). RVP now has
the resampling condition, the AC3 bitrate and the selection method; the
normalizer's two other methods keep a reason. Defaults read there:
Crossfeed 700 Hz and 6 dB (RVP's feed was 4.5 dB), Noise Reduction's
threshold 100 of 1 to 5000, Extender's HPF 1 to 3 (RVP took 0 to 100),
Freeverb's and Crystality's defaults as RVP's.
