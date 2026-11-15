# Goal U status: the Linux audit's fixes

Started 2026-11-14 on the owner's Ubuntu 26.10 boot (GNOME 51 on Wayland,
RTX 50-series card, NVIDIA 615.71.09). The findings are those of
`docs/audits/linux.md` (H1 to H3, M1 to M16, L1 to L27); the work plan
is its last section. Goal U-W checks on the Windows VM what reaches
Windows ("For goal U-W" below).

## The owner's decisions (2026-11-14)

The owner answered the audit's questions in one round, each with the
recommended option:

1. **H1:** CI builds and caches whisper.cpp, MVTools and RVP's
   VapourSynth runtime for its Linux packages.
2. **H3:** the overlapping libraries move to `SYSTEM` (Depends); RVP's
   binary keeps its RPATH.
3. **M2:** with the media transport controls off, the screen kept on, the
   launcher's progress and pausing on lock still work; only MPRIS goes.
4. **M5, L12, L13:** the cases of several local accounts are fixed now.
5. **L15:** saved frames and recordings go to the XDG user folders
   (`user-dirs.dirs`), `~/Pictures/RVP` and `~/Videos/RVP` without them.
6. **L3:** the calibration's key is the content of the engines' files,
   not their size and time.
7. **M14:** the AUR package uses the system's VapourSynth with optdepends.
8. **m6svp:** the second window's check expects RVP's diagnostic (SVP's
   channel taken), as on Windows.

## Packages

| # | Package | Result |
|---|---------|--------|
| 0 | Start | The boot run's report, its status sections, the audit and goal U's entry committed with dbde0cd and c3f58dc, and pushed |
