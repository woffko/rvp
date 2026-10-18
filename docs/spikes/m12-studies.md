# M12 studies

Goal N (milestone M12) asked for studies before some parts go in: Record
Video in a second core, 3D subtitles, H.264 MVC, Desktop Mode, the
AviSynth bridge and D4's Direct3D 11 child window. It also needed a way
to run PotPlayer's pixel shaders in mpv. This file records what each
study found and what RVP does about it. The menu items and options that
stay out keep these reasons (`crates/app/src/coverage.rs`,
`docs/coverage/`).

## Record Video: encoding in a second core

PotPlayer's Video Recording records the picture as RVP shows it: the
video with its filters. libmpv has mpv's encoding mode (the `o` option):
a core given an output file decodes its file, runs its filters and writes
the result instead of playing it.

Probe (2026-10-18, Ubuntu's libmpv 0.37, FFmpeg 6.1): a second core
created after the player's core, with `o`, `start`, `end`, `vf`
(`lavfi=[hflip]`), `ovc=libx264` and `oac=aac`, loading the 60 s
fixture, wrote a valid Matroska file of the stretch asked for (H.264
96x72 and AAC, 3.03 s for 2 s to 5 s) in about half a second. The file
is complete only once the core has quit; until then it can be empty. On
Windows (the VM, shinchiro's libmpv) dropping the handle at the end of
the file left it empty: a script's client kept the core alive after
`mpv_destroy` returned. RVP sends `quit` and waits for the core's
`Shutdown` event, after which the file is written (62 KB for the test's
stretch on the VM).

What RVP does (`crates/engine/src/encode.rs`, `crates/app/src/record.rs`):

- Record Video... (Alt+C) asks for a file and marks the time it starts;
  the same command, or Start/Stop Video Recording, stops it. Then a core
  of its own, on a thread of its own, encodes the file from the start mark
  to the stop with RVP's video filters (never SVP's `@svp`): H.264 and AAC
  for `.mkv`, `.mp4` and `.mov`, VP9 and Opus for `.webm`.
- It records the source again rather than the screen: the picture is the
  one RVP's filters make, at the video's own size, without the renderer's
  colour controls, shaders or SVP's frames, and a live stream cannot be
  read again (Record Stream records it as it comes).
- `tests/m12.rs` (`streams_and_videos_are_recorded`) checks the length of
  a recording; `m12-all.ps1` repeats it with the Windows libmpv.

Record Stream needs no study: mpv's `stream-record` writes the packets
the demuxer reads from the moment it is set, the container by the file's
extension, and finishes the file when it is cleared (the recorder starts at
a keyframe). RVP ends it with its file.

## PotPlayer's pixel shaders in mpv

PotPlayer's `PxShader` folder holds MPC-HC's single-pass HLSL shaders
(`ps_2_0` and `ps_3_0`): `sampler s0`, the constants `p0` (width, height,
counter, clock), `p1` (1/width, 1/height) and `p2` (the video's rectangle
in the texture), `tex2D`. mpv runs GLSL hooks. RVP translates a shader
when it loads (`crates/app/src/hlsl.rs`): a prelude of macros maps HLSL's
types and functions onto GLSL's, and a few rewrites do the rest
(semantics, attributes, brace initializers, a vector set from a scalar, a
scalar returned as a vector, the entry point's name, the global `c0`).
Matrices stay consistent by storing them transposed and turning
`mul(a, b)` into `b * a`.

`rvp --selftest=shaders` puts each shader of the catalogue in its list in
turn and reads mpv's log for compile errors. On 2026-10-18 all 34 of the
owner's installed PotPlayer shaders (read in place, never copied) and
RVP's eleven own ones compiled with mpv's OpenGL renderer (Mesa, GLSL
4.40) under Xvfb; grabs showed Invert inverting, GrayScale grey and
HorzFlip flipping. A shader that needs what mpv's hooks lack is refused
with its reason: a second texture (`s1`), constants after `c2`,
projected or gradient sampling.

glslang's HLSL front end with SPIRV-Cross, the fallback PLAN.md named,
was not needed.

## 3D subtitles

PotPlayer's Stereo 3D Subtitle Mode draws a subtitle once for each eye of
a side-by-side or top-and-bottom picture, with a depth offset. mpv draws
subtitles (libass) after the video filters, once over the whole picture,
and has no stereo subtitle mode (the old `--stereo-mode` is gone).

The way that would work: render the subtitles inside RVP's filter graph,
with FFmpeg's `subtitles` filter (libass) once per eye before the eyes are
joined, shifted by the depth, and turn mpv's own subtitles off. Its limits
are why it is not in RVP:

- the filter renders text subtitles only (SRT, ASS, WebVTT); a 3D
  Blu-ray's subtitles are pictures (PGS);
- it reads the subtitle file itself, so RVP's subtitle settings (style,
  sync, position, the second subtitle) and track changes would need a
  graph rebuild each time, which mpv's own renderer does without one;
- every subtitle change rebuilds the video chain (a short stall).

The Stereo 3D Subtitle Mode items keep the reason: mpv draws subtitles
once over the finished picture.

## H.264 MVC (3D Blu-ray)

A 3D Blu-ray's video is H.264 MVC: the base view is ordinary H.264 and
the second view an extension. FFmpeg's H.264 decoder decodes only the base
view, and libmpv decodes through FFmpeg, so RVP shows the left eye of
such a file in 2D. FFmpeg 7.1 and later decode MV-HEVC (the other
multi-view codec), but mpv takes one view of it. PotPlayer's H.264 MVC 3D
items keep this reason; Matroska files with both views side by side or
top and bottom play in 3D through `stereo3d`.

## Desktop Mode

PotPlayer's Desktop Mode (Shift+Enter) plays the video behind the desktop
icons, as a wallpaper. On Windows that means moving the window into
Explorer's desktop layer: an undocumented message (0x052C) to the
`Progman` window makes Explorer create a `WorkerW` window behind the icons,
and the player's window becomes its child (`SetParent`). Wallpaper
programs do this, with three problems for RVP:

- the message and the window tree are Explorer's internals, and Windows
  11 24H2 changed them (the `WorkerW` became a child of `Progman`), so the
  same code has to find the layer in two ways and may break again;
- the window then gets no keyboard or mouse input (the icons are in
  front), so RVP would need a second way to leave the mode (the tray, a
  global hotkey) and would show no skin;
- RVP's whole Qt window would move, with its OpenGL context, into a
  window of another process; a crash leaves the wallpaper black until
  Explorer redraws it.

Desktop Mode is not in RVP; its menu item and key keep this reason. On
Linux each desktop has its own wallpaper mechanism (none on GNOME Wayland
for a client window), so goal S lists it as not possible there.

## AviSynth

Three parts, three answers:

- **Opening `.avs` scripts like files.** FFmpeg reads them through its
  `avisynth` demuxer, which loads the user's AviSynth+ at run time, when
  FFmpeg is built with it. RVP lists `.avs` as a media file; whether the
  Windows libmpv's FFmpeg has the demuxer is printed by
  `tests/m12.rs` (`ffmpeg_offers_what_m12_reads_and_writes`) on the
  Windows VM. Ubuntu's FFmpeg 6.1 has neither the AviSynth nor the
  VapourSynth demuxer (Linux: goal S).
- **AviSynth plugins in VapourSynth scripts.** VapourSynth's AviSynth
  compatibility (`core.avs.LoadPlugin`, Windows) loads them; RVP's script
  wrapper loads the plugins the settings list (`vapoursynth.avs-plugins`)
  before the script.
- **PotPlayer's AviSynth processing of the playing video.** PotPlayer
  feeds the playing video into an AviSynth script (`potplayer_source()`).
  mpv has no AviSynth filter; the only way is RVP's VapourSynth slot with
  a bridge that runs AviSynth inside VapourSynth. The bridge that exists,
  avsw (`avsw.Eval`, the VapourSynth clip injected under a name), runs a
  32-bit AviSynth in a second process through avsproxy, takes 8-bit video
  only and copies every frame between the processes. That is a third-party
  binary RVP would have to ship or ask for, for 8-bit video at a cost per
  frame, to run PotPlayer's presets, which are mostly MVTools motion
  interpolation (the TrueMotion scripts) that goal T brings natively. RVP
  does not use it: the AviSynth processing items and options keep this
  reason, and VapourSynth scripts (with AviSynth plugins in them) take
  their place.

## Direct3D 11 output (D4)

Written when the study's host session is done.
