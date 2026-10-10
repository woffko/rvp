# Spike S5: normalizer parity with PotPlayer

Question (PLAN.md M0): which filter chain and parameters match PotPlayer's
normalizer with boost (Level 75 %, Release time 10), compared by EBU R128
short-term loudness over the same clips?

Answer: **`dynaudnorm=f=5000:g=3:p=1.0:m=5`** comes closest among dynaudnorm
chains. It halves the difference from PotPlayer: short-term loudness differs
by 2.2 LU RMS once PotPlayer's slow start is over (2.8 LU including it),
against 5.3-5.5 LU for no normalizer. `speechnorm`, an `acompressor` used as
an AGC and a simple AGC model with fast attack and a fixed release rate do
not do better. The remaining difference is mostly PotPlayer's asymmetric
behaviour (fast attack, slow release), which dynaudnorm's centred smoothing
cannot reproduce. The plan's fallback, a blind A/B listening test by the
owner, decides in M3 whether that is close enough.

## Setup

- PotPlayer 260819 on the owner's Windows host, as configured (confirmed on
  the Preferences page, `docs/r1/README.md`): normalizer on for all files,
  method "Tomsteady 32-bit floating point arithmetic", Boost audio on,
  Level 75 %, Release time 10; PotPlayer volume as set by the owner (22 %).
  No PotPlayer setting was changed.
- Clips (synthetic, owner decision; `tools/test-clips/make-clips.sh`,
  48 kHz stereo FLAC):

  | Clip | Content | Source loudness |
  |------|---------|-----------------|
  | `audio-dialogue-action` | 30 s cycles: 20 s of quiet speech-like band-limited noise with a syllable envelope, 10 s of loud pink noise with low hits | -36 / -14 LUFS short-term, I -14.6 LUFS, LRA 22.5 LU |
  | `audio-music` | A major chord with harmonics: pp 30 s, crescendo 30 s, ff 10 s, sudden pp 10 s, ff 10 s | -47 → -14 LUFS, I -15.7 LUFS, LRA 13.7 LU |
  | `audio-steps` | Pink noise at four levels (10 s each), 1 kHz bursts, 10 s of silence, the lowest level again | -59 / -48 / -39 / -29 LUFS |

- Recording: `host-session s5` captured the default output device (FiiO Q
  series over S/PDIF, 44.1 kHz) with WASAPI loopback while PotPlayer played
  each clip, one PotPlayer start per clip. Nothing else played.
- "Without normalizer" reference: the source itself, decoded by ffmpeg
  (owner decision, so no PotPlayer setting had to be switched).
- Analysis: `tools/s5/analyze.py`. Each recording is aligned with its source
  (first sound, then cross-correlation within ±0.5 s; correlation 0.93-0.995;
  the loopback stream starts with PotPlayer's audio, offset 0.03 s). The
  short-term loudness S (3 s window, every 100 ms) of a candidate chain
  applied to the source is compared with S of the recording. One constant
  offset for all three clips is removed (PotPlayer's volume is the same for
  all), then the RMS difference in LU is averaged over the clips with equal
  weight. Only digital silence is left out.

## What PotPlayer's normalizer does

Short-term loudness of source and recording, every 5 s (recording about
30 LU lower because of PotPlayer's volume):

```text
dialogue  20 s -35.8 -> -55.2 | 25 s -14.4 -> -44.8 | 35 s -35.6 -> -65.0 | 40 s -36.8 -> -61.8 | 45 s -36.4 -> -57.8 | 50 s -35.8 -> -54.5
music     30 s -47.0 -> -62.6 | 40 s -24.4 -> -39.9 | 50 s -18.0 -> -40.0 | 60 s -14.3 -> -40.2 | 75 s -47.0 -> -73.4 | 80 s -47.0 -> -70.3
steps      5 s -59.0 -> -90.3 | 10 s -59.1 -> -85.8 | 25 s -39.0 -> -56.5 | 35 s -28.6 -> -43.8 | 65 s -59.0 -> -76.7
```

- **Fast attack, slow release.** When the level jumps up, the gain drops at
  once. After a loud passage the gain climbs back at about 0.6-0.7 dB per
  second (Release time 10).
- **Tight target.** The music crescendo rises by 17 LU in the source and by
  less than 1 LU in the recording once the target is reached.
- **Bounded range.** The gain (recording minus source) moves between about
  -31 and -15 dB here, so quiet passages are lifted by up to about 16 dB
  relative to loud ones. The absolute boost cannot be separated from
  PotPlayer's volume in these recordings.
- **Slow start.** Each file starts at the low end of that range, as after a
  loud passage, and ramps up at the release rate over the first 10-20 s.

## Results

Joint RMS difference in LU (one offset for all clips; per clip: dialogue
/ music / steps). "From 15 s" leaves out the first 15 s of each clip, where
PotPlayer's gain is still ramping up.

| Chain | All frames | From 15 s | Per clip, from 15 s |
|-------|------------|-----------|---------------------|
| No normalizer (the source) | 5.48 | 5.28 | |
| **`dynaudnorm=f=5000:g=3:p=1.0:m=5`** | **2.80** | **2.22** | 2.00 / 2.02 / 2.18 |
| `dynaudnorm=f=6000:g=3:p=1.0:m=5` (best with all frames) | 2.76 | 2.41 | 3.19 / 2.14 / 2.07 |
| `dynaudnorm=f=1500:g=9:p=1.0:m=4` (best with frames up to 1.5 s) | 3.41 | 2.94 | 3.41 / 2.21 / 2.61 |
| `dynaudnorm=f=5000:g=3:p=1.0:m=4` | 2.81 | 2.36 | 2.03 / 2.14 / 1.99 |
| `dynaudnorm` defaults (`f=500:g=31:p=0.95:m=10`) | 5.30 | 4.42 | 4.46 / 4.00 / 4.83 |
| `speechnorm=p=0.95:e=5:c=10:r=0.0001:f=0.001` (best from 15 s) | 4.45 | 2.91 | 3.27 / 1.71 / 1.65 |
| `acompressor=threshold=0.03:ratio=4:attack=80:release=9000:makeup=4:detection=rms` (best of 576 as an AGC) | 6.49 | 4.66 | |
| AGC model (`agc_model.py`): instant attack, 0.5 dB/s release, 16 dB range | 2.99 | 3.23 | |

Search (`tools/s5/analyze.py`): about 2300 coarse dynaudnorm candidates
(frame 100-1000 ms, Gaussian 3-51, peak 0.5-0.95, max gain 3-100, target
RMS 0-0.25), then finer grids that followed the optimum to long frames
(1.5-8 s), the smallest Gaussian window (3) and peak 1.0; max gain settled
at 5 (4 and 6 are worse). 1024 speechnorm and 576 acompressor candidates.
The best chains reproduce a simulated recording exactly (0.00 LU), which
checks the alignment and scoring.

Two artefacts of the measurement: PotPlayer's volume (22 %) puts the
recording about 30 LU below the source, which the common offset removes;
and the absolute boost cannot be measured, only the shape of the gain over
time.

## Recommendation for M3

1. Start with `dynaudnorm=f=5000:g=3:p=1.0:m=5` as the normalizer filter
   (mpv `af`) for PotPlayer's defaults (Boost on, Level 75 %, Release
   time 10). Boost maps to the maximum gain (`m=5`, up to +14 dB; off =
   `m=1`). How other Level and Release values map to `p` and `f` is tuned in
   M3 by ear. Volume above 100 % uses mpv's `volume-max`, as planned.
2. Let the owner compare RVP and PotPlayer blind on the owner's own
   material (the fallback in PLAN.md) before the chain is final.
3. If the slow release matters in that test, replace the filter with an
   RVP-side AGC: measure loudness with lavfi `ebur128` in mpv's filter chain
   and drive mpv's volume from Rust with an instant attack and a release of
   about 0.5-0.7 dB/s. The simple model in `agc_model.py` shows the shape is
   right but needs a better detector before it beats dynaudnorm.
