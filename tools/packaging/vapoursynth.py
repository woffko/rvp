#!/usr/bin/env python3
"""Assembles RVP's own VapourSynth runtime for Windows (goal T, D18), which
the Windows packages carry in `vapoursynth/` next to rvp.exe and RVP points
libmpv at (never SVP's runtime):

- VapourSynth R73 portable on an embeddable Python 3.12 (as spike S2 used
  it), with MVTools v24 in `vs-plugins` (loaded by every script: M18's
  smoothing and PotPlayer's motion interpolation script);
- RIFE's ncnn-Vulkan plugin (r9_mod_v33) with model v4.6 in `rife/`,
  loaded by RVP's RIFE script only, with Microsoft's OpenMP runtime
  (vcomp140.dll) next to it;
- vs-mlrt v15.15's TensorRT plugin (vstrt.dll), its trtexec and vsmlrt.py,
  with RIFE v4.6's ONNX model, in `vs-mlrt/` (not loaded on its own): RIFE
  (TensorRT) copies them next to NVIDIA's TensorRT, which RVP downloads
  from NVIDIA only when the user asks (crates/app/src/tensorrt.rs; NVIDIA's
  licence lets RVP's packages carry none of it).

Every download is checked by its SHA-256 and kept in --cache. The output
folder is made anew; NOTICE.txt there names each part and its licence.

usage: vapoursynth.py --out DIR --vcomp vcomp140.dll [--cache DIR] [--sevenzip 7z]
"""

import argparse
import hashlib
import shutil
import subprocess
import sys
import urllib.request
import zipfile
from pathlib import Path

RIFE_TAG = "r9_mod_v33"
VSMLRT_TAG = "v15.15"
VSMLRT = f"https://github.com/AmusementClub/vs-mlrt/releases/download/{VSMLRT_TAG}"
RIFE_RAW = f"https://raw.githubusercontent.com/styler00dollar/VapourSynth-RIFE-ncnn-Vulkan/{RIFE_TAG}"
MODEL = "rife-v4.6_ensembleFalse"

# (file in the cache, URL, SHA-256)
DOWNLOADS = {
    "vapoursynth": (
        "VapourSynth64-Portable-R73.zip",
        "https://github.com/vapoursynth/vapoursynth/releases/download/R73/VapourSynth64-Portable-R73.zip",
        "3326f10d0fdcdec45649a474cbc9810795ab3da422634d0f134bca6089afbb91",
    ),
    "python": (
        "python-3.12.9-embed-amd64.zip",
        "https://www.python.org/ftp/python/3.12.9/python-3.12.9-embed-amd64.zip",
        "615861fb801e8b04c847598db4e1e46e4b046295017caa37cb5486dde72b5865",
    ),
    "mvtools": (
        "vapoursynth-mvtools-v24-win64.7z",
        "https://github.com/dubhater/vapoursynth-mvtools/releases/download/v24/vapoursynth-mvtools-v24-win64.7z",
        "b9883003eed100d4ffd44344b658554076b89da6d45041b378e675720426bc95",
    ),
    "rife": (
        f"librife_windows_x86-64-{RIFE_TAG}.dll",
        f"https://github.com/styler00dollar/VapourSynth-RIFE-ncnn-Vulkan/releases/download/{RIFE_TAG}/librife_windows_x86-64.dll",
        "36a25b471be88e6f915320c818022dc8657dd9beac22a8c3158bd7f4260cc410",
    ),
    "flownet.bin": (
        f"{MODEL}-flownet.bin",
        f"{RIFE_RAW}/models/{MODEL}/flownet.bin",
        "03393cf14fe6d0af015d24d93bae1e5925f2828cb3b6555300a132643c617aa5",
    ),
    "flownet.param": (
        f"{MODEL}-flownet.param",
        f"{RIFE_RAW}/models/{MODEL}/flownet.param",
        "ae9b08af43fba97e27aa2c40c04c35b1a75f637b2d2b01a941fa94361ab25cbd",
    ),
    "vstrt": (
        f"VSTRT-Windows-x64.{VSMLRT_TAG}.7z",
        f"{VSMLRT}/VSTRT-Windows-x64.{VSMLRT_TAG}.7z",
        "d44cef26df68745b9b6474cfe078b9baeffaac80b8201bc9f34d01ba9557d60d",
    ),
    "vsmlrt-scripts": (
        f"vsmlrt-scripts.{VSMLRT_TAG}.7z",
        f"{VSMLRT}/scripts.{VSMLRT_TAG}.7z",
        "567abe5b9890229bc0ee026dc14fda3fc99a1a8260e6fc0db7878831961b1b21",
    ),
    "vsmlrt-licence": (
        f"vs-mlrt-{VSMLRT_TAG}-LICENSE",
        f"https://raw.githubusercontent.com/AmusementClub/vs-mlrt/{VSMLRT_TAG}/LICENSE",
        "3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986",
    ),
    # RIFE v4.0 to v4.10 for vs-mlrt (model-20220923; no SHA-256 published:
    # taken on 2026-11-07, the archive's own CRCs checked).
    "rife-onnx": (
        "vs-mlrt-rife_v8.7z",
        "https://github.com/AmusementClub/vs-mlrt/releases/download/model-20220923/rife_v8.7z",
        "1a18614e72164c61d60861199fc8be6b0003110f747646b3dafb9acd632f6ef7",
    ),
}

# Parts of the portable archive a player does not need: its own archiver,
# the file mounter and its installer, the documentation, the SDK, VSPipe,
# the VFW bridge, the Python 3.8 loader and the wheels (unpacked below).
UNNEEDED = [
    "7z.dll", "7z.exe", "AVFS.exe", "pfm-192-vapoursynth-win.exe", "MANIFEST.in", "doc", "sdk",
    "vsgenstubs.py", "vsgenstubs4", "vsrepo.py", "VSPipe.exe", "VSVFW.dll", "VSScriptPython38.dll",
    "wheel", "python.exe", "pythonw.exe",
]

NOTICE = """RVP's VapourSynth runtime (motion smoothing, goal T; never SVP's)

VapourSynth R73 (portable)        LGPL-2.1-or-later   https://github.com/vapoursynth/vapoursynth
Python 3.12.9 (embeddable)        PSF License 2.0     https://www.python.org/ (LICENSE.txt here)
MVTools v24 (vs-plugins)          GPL-2.0-or-later    https://github.com/dubhater/vapoursynth-mvtools
RIFE ncnn-Vulkan {rife} (rife/)  MIT                 https://github.com/styler00dollar/VapourSynth-RIFE-ncnn-Vulkan (rife/LICENSE)
  with ncnn                       BSD-3-Clause        https://github.com/Tencent/ncnn
  model {model}     MIT                 RIFE, https://github.com/hzwer/Practical-RIFE
vcomp140.dll (rife/)              Microsoft Visual C++ redistributable
vs-mlrt {vsmlrt} (vs-mlrt/)        GPL-3.0             https://github.com/AmusementClub/vs-mlrt (vs-mlrt/LICENSE)
  trtexec.exe                     Apache-2.0          TensorRT's sample, built by vs-mlrt
  models/rife/rife_v4.6.onnx      MIT                 RIFE, https://github.com/hzwer/Practical-RIFE
NVIDIA's TensorRT is not part of this package: RVP downloads it from NVIDIA
on the user's request, under NVIDIA's licence.
""".format(rife=RIFE_TAG, model=MODEL, vsmlrt=VSMLRT_TAG)


def fetch(cache: Path, key: str) -> Path:
    name, url, sha256 = DOWNLOADS[key]
    target = cache / name
    if not target.exists() or hashlib.sha256(target.read_bytes()).hexdigest() != sha256:
        print(f"download {url}")
        request = urllib.request.Request(url, headers={"User-Agent": "rvp-packaging"})
        with urllib.request.urlopen(request, timeout=120) as response:
            data = response.read()
        digest = hashlib.sha256(data).hexdigest()
        if digest != sha256:
            raise SystemExit(f"{url}: SHA-256 {digest}, expected {sha256}")
        target.write_bytes(data)
    return target


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--out", required=True)
    parser.add_argument("--vcomp", required=True, help="vcomp140.dll (Microsoft's OpenMP runtime)")
    parser.add_argument("--cache", default=str(Path.home() / ".cache" / "rvp-vapoursynth"))
    parser.add_argument("--sevenzip", default="7z")
    args = parser.parse_args()
    cache = Path(args.cache)
    cache.mkdir(parents=True, exist_ok=True)
    out = Path(args.out)
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)

    zipfile.ZipFile(fetch(cache, "python")).extractall(out)
    with open(out / "python312._pth", "a", newline="\r\n") as pth:
        pth.write("vs-scripts\nLib\\site-packages\n")
    zipfile.ZipFile(fetch(cache, "vapoursynth")).extractall(out)
    (out / "Lib" / "site-packages").mkdir(parents=True, exist_ok=True)
    (out / "vs-scripts").mkdir(exist_ok=True)
    wheel = next((out / "wheel").glob("*abi3*.whl"))
    zipfile.ZipFile(wheel).extractall(out / "Lib" / "site-packages")
    # pip would install the core library from the wheel's data directory.
    data = next((out / "Lib" / "site-packages").glob("vapoursynth-*.data"))
    shutil.move(str(data / "data" / "Lib" / "site-packages" / "vapoursynth.dll"),
                str(out / "Lib" / "site-packages" / "vapoursynth.dll"))
    shutil.rmtree(data)
    for name in UNNEEDED:
        path = out / name
        if path.is_dir():
            shutil.rmtree(path)
        elif path.exists():
            path.unlink()

    mvtools = cache / "mvtools"
    if not (mvtools / "libmvtools.dll").exists():
        subprocess.run([args.sevenzip, "x", "-y", f"-o{mvtools}", str(fetch(cache, "mvtools"))],
                       check=True, stdout=subprocess.DEVNULL)
    (out / "vs-plugins").mkdir(exist_ok=True)
    shutil.copy2(mvtools / "libmvtools.dll", out / "vs-plugins" / "libmvtools.dll")

    rife = out / "rife"
    model = rife / "models" / MODEL
    model.mkdir(parents=True)
    shutil.copy2(fetch(cache, "rife"), rife / "librife_windows_x86-64.dll")
    shutil.copy2(fetch(cache, "flownet.bin"), model / "flownet.bin")
    shutil.copy2(fetch(cache, "flownet.param"), model / "flownet.param")
    shutil.copy2(args.vcomp, rife / "vcomp140.dll")
    # The plugin's other runtime library, next to it as well: Windows looks
    # for a plugin's libraries in the plugin's folder.
    shutil.copy2(out / "msvcp140_atomic_wait.dll", rife / "msvcp140_atomic_wait.dll")
    licence = urllib.request.urlopen(
        urllib.request.Request(f"{RIFE_RAW}/LICENSE", headers={"User-Agent": "rvp-packaging"}), timeout=60
    ).read()
    (rife / "LICENSE").write_bytes(licence)

    # vs-mlrt: the plugin and trtexec, vsmlrt.py, and RIFE v4.6's model only.
    vsmlrt = out / "vs-mlrt"
    (vsmlrt / "models" / "rife").mkdir(parents=True)
    unpacked = cache / f"vs-mlrt-{VSMLRT_TAG}"
    if not (unpacked / "vstrt.dll").exists():
        subprocess.run([args.sevenzip, "x", "-y", f"-o{unpacked}", str(fetch(cache, "vstrt"))],
                       check=True, stdout=subprocess.DEVNULL)
        subprocess.run([args.sevenzip, "x", "-y", f"-o{unpacked}", str(fetch(cache, "vsmlrt-scripts"))],
                       check=True, stdout=subprocess.DEVNULL)
        subprocess.run([args.sevenzip, "x", "-y", f"-o{unpacked}", str(fetch(cache, "rife-onnx")),
                        "rife/rife_v4.6.onnx"], check=True, stdout=subprocess.DEVNULL)
    shutil.copy2(unpacked / "vstrt.dll", vsmlrt / "vstrt.dll")
    shutil.copy2(unpacked / "vsmlrt-cuda" / "trtexec.exe", vsmlrt / "trtexec.exe")
    shutil.copy2(unpacked / "vsmlrt.py", vsmlrt / "vsmlrt.py")
    shutil.copy2(unpacked / "rife" / "rife_v4.6.onnx", vsmlrt / "models" / "rife" / "rife_v4.6.onnx")
    shutil.copy2(fetch(cache, "vsmlrt-licence"), vsmlrt / "LICENSE")
    (out / "NOTICE.txt").write_text(NOTICE, encoding="utf-8", newline="\r\n")

    files = sorted(p for p in out.rglob("*") if p.is_file())
    size = sum(p.stat().st_size for p in files)
    print(f"vapoursynth: {len(files)} files, {size / 1e6:.1f} MB in {out}")


if __name__ == "__main__":
    sys.exit(main())
