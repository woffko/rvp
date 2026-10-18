# RVP's default shaders

mpv GLSL hooks that RVP ships, so that the Pixel Shaders lists have
something without PotPlayer (owner request, 2026-10-18; PLAN.md M12).
`crates/app/src/shaders.rs` builds them into RVP and writes them to the
state folder (`shaders/rvp`) when they are used. Each keeps its own
license header; `LICENSE-LGPL-3.0.txt` is the LGPL's text for the
LGPL-licensed ones. Every license here is GPL-3.0-compatible (D8).

They were fetched on 2026-10-18 and are unchanged. The stage is where
mpv runs the shader (its `//!HOOK` points): RVP lists it under the
matching PotPlayer list.

| File | What it does | Stage | Origin | License |
|------|--------------|-------|--------|---------|
| `FSR.glsl` | AMD FidelityFX Super Resolution 1.0.2 (EASU upscaler, RCAS sharpening) | pre-resize (`LUMA`) | agyild's port, https://gist.github.com/agyild/82219c545228d70c5604f865ce0b0ce5 | MIT (AMD) |
| `CAS.glsl` | AMD FidelityFX Contrast Adaptive Sharpening | pre-resize (`LUMA`) | agyild's port, https://gist.github.com/agyild/bbb4e58298b2f86aa24da3032a0d2ee6 | MIT (AMD) |
| `CAS-scaled.glsl` | AMD CAS with upsampling | pre-resize (`LUMA`) | the same gist | MIT (AMD) |
| `NVScaler.glsl` | NVIDIA Image Scaling 1.0.2 (scaler and sharpening) | pre-resize (`LUMA`) | agyild's port, https://gist.github.com/agyild/7e8951915b2bf24526a9343d951db214 | MIT (NVIDIA) |
| `NVSharpen.glsl` | NVIDIA Image Sharpening 1.0.2 | pre-resize (`LUMA`) | the same gist | MIT (NVIDIA) |
| `FSRCNNX_x2_8-0-4-1.glsl` | FSRCNNX, a small neural network that doubles the luma | pre-resize (`LUMA`) | igv, https://github.com/igv/FSRCNN-TensorFlow/releases/tag/1.1 | LGPL-3.0-or-later |
| `ravu-lite-ar-r4.hook` | RAVU-Lite with anti-ringing, a fast luma doubler | pre-resize (`LUMA`) | bjin, https://github.com/bjin/mpv-prescalers | LGPL-3.0 |
| `KrigBilateral.glsl` | chroma upscaling guided by the luma | pre-resize (`CHROMA`) | igv, https://gist.github.com/igv/a015fc885d5c22e6891820ad89555637 | LGPL-3.0-or-later |
| `SSimSuperRes.glsl` | corrects the upscaled picture toward the source | post-resize (`POSTKERNEL`) | igv, https://gist.github.com/igv/2364ffa6e81540f29cb7ab4c9bc05b6b | LGPL-3.0-or-later |
| `SSimDownscaler.glsl` | a sharper, perceptual downscaler | post-resize (`POSTKERNEL`) | igv, https://gist.github.com/igv/36508af3ffc84410fe39761d6969be10 | LGPL-3.0-or-later |
| `adaptive-sharpen.glsl` | bacondither's adaptive sharpening | post-resize (`OUTPUT`) | igv's port, https://gist.github.com/igv/8a77e4eb8276753b54bb94c1c50c317e | BSD-2-Clause (bacondither) |
