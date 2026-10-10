# Goal C status: Windows host session (S1, S3 performance, R1, S5)

Date: 2026-10-10. Result: done. **Windows gate: passed** (verdict below).

Every step that started a program or captured the screen on the owner's
desktop was announced and ran only after the owner's explicit yes. All test
clips are synthetic (owner decision): `tools/test-clips/make-clips.sh`.
Captures, recordings and raw logs stay in `~/rvp-reference`.

## Summary

| Item | Result | Details |
|------|--------|---------|
| S1: SVP attaches | **PASS.** SVP 4.7.0 attached to the libmpv spike (process not named `mpv`) in 1.5 s and interpolated 23.976 → 59.94 fps | `docs/spikes/s1-svp-attach.md` |
| S3 performance | **PASS with a rule.** 4K60 HEVC, RTX 30-series card: 0 dropped frames with the OpenGL item when chrome updates are applied together with the next video frame; 7-15 drops per 30 s when the chrome updates on its own; 0 drops with mpv in a child window (`wid`) | `docs/spikes/s3-qt-shell.md` |
| S3 real input on Windows | **PASS.** Real mouse drags move (80,60) and resize (100,80) the frameless window; the context menu is a 264x509 window over a 157-pixel-high window | `docs/spikes/s3-qt-shell.md` |
| R1 | **Done.** 18 window captures (idle and playing × 3 sizes × plain/hover/pressed) plus the menu and Preferences; context menu (894 static items, 1580 live); default hotkeys (259 + 54); mouse-action table; normalizer settings confirmed | `docs/r1/` |
| S5 | **Done, partial parity.** Loopback recordings of PotPlayer's normalizer (Tomsteady, Boost, 75 %, 10) on three synthetic programs. Best dynaudnorm chain `dynaudnorm=f=5000:g=3:p=1.0:m=5`: 2.2 LU RMS short-term loudness difference after PotPlayer's slow start (2.8 LU including it), against 5.3-5.5 LU without a normalizer; speechnorm, an acompressor AGC and a simple AGC model are not better. The rest is PotPlayer's fast-attack/slow-release asymmetry; the owner's blind A/B test in M3 decides | `docs/spikes/s5-normalizer.md` |

## Gate verdict for Windows

PLAN.md: "confirm D1-D5 or switch to the documented fallback before M2".

| Decision | Verdict | Evidence |
|----------|---------|----------|
| D1 libmpv via `libmpv2` | Confirmed | S1 (SVP attaches through `mpvpipe`), S2 (VapourSynth through a run-time `get_core` wrapper, owner decision), S3 (render API) |
| D2 own skin engine for `.dsf` | Confirmed | S4 (goal A); R1 captures for golden images; the spike paints BlackSpace from Rust |
| D3 Qt 6 via cxx-qt, QML | Confirmed | Frameless move/resize and a menu beyond the window with real input on Windows (and on GNOME Wayland in goal B) |
| D4 OpenGL render API first; `wid` optional on Windows | Confirmed, with a rule | 0 drops at 4K60 when chrome updates ride on video frames; `wid` (0 drops without that rule) stays the Windows fallback |
| D5 display list painted with `QPainter` in a Rust `QQuickPaintedItem` | Confirmed | 5.1 ms for the whole 1920x1080 skin, 2.1 ms for a label layer |

S5 does not gate D1-D5; its chain is the starting point for M3's
normalizer.

Rules that M2 inherits from the gate:

1. During playback every chrome change goes through one path that applies
   it with mpv's next frame; the skin is split into a static layer and
   small dynamic items.
2. PotPlayer's `.vpy` scripts run through the generated `get_core` wrapper.
3. With `wid`, the Qt thread never blocks on mpv.
4. RVP recognises SVP's filter by its `@svp` label and leaves it alone.

## Host-side effects and cleanup

- PotPlayer saves its window state on exit. The `Positions` key was
  snapshotted before every PotPlayer session and restored afterwards with
  the owner's permission; it is identical to the snapshot. PotPlayer's
  `MInfo1`/`MInfo2` and root `(Default)` values change on every start and
  were left as PotPlayer wrote them.
- The synthetic clips are in PotPlayer's Default playlist and history; the
  owner removes them (owner decision).
- SVP built and cached a TensorRT engine for RIFE on first use (kept at the
  owner's choice).
- Test files are in `C:\Users\<user>\rvp-test` (bundle, clips, outputs).
- An injected Esc key interrupted one R1 session (it reached the terminal
  that ran the tool) and left PotPlayer open; the owner closed it and the
  window state was restored. The tool now injects only mouse input and
  drives menus and dialogs with window messages.

## Open questions for the owner

1. **License rule (open since goal A).** Confirm that the `rust` skill's
   no-GPL/LGPL rule is waived for libmpv and Qt in this project before M2.
2. **`C:\Users\<user>\rvp-test`.** Keep it for goal D and later Windows checks,
   or remove it now?
