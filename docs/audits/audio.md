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
  level.
- The normalizer's block per source (finding 2).
- Which of the missing functions to add, and in what order (the owner's
  choice).
