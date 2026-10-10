# RVP compatibility shim: VapourSynth R55+ removed vs.get_core(), which
# PotPlayer's scripts still call. Runs once when the embedded Python starts.
import vapoursynth

if not hasattr(vapoursynth, "get_core"):
    def get_core(threads=None, add_cache=None):
        return vapoursynth.core

    vapoursynth.get_core = get_core
