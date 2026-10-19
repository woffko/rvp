# RVP's libmpv for Windows

RVP's Windows packages used shinchiro's libmpv builds
(`mpv-dev-x86_64-*-git-*.7z` from shinchiro/mpv-winbuild-cmake's releases).
Those builds configure FFmpeg without a SPIR-V compiler, so FFmpeg leaves
out its Vulkan compute filters, and with them `fruc_vulkan` (NVIDIA's
optical flow), M18's fourth smoothing engine (D18). `build.sh` builds
libmpv with the same recipe at a fixed commit plus one change,
`ffmpeg-libshaderc.patch` (`--enable-libshaderc`; the recipe already builds
shaderc for libplacebo and mpv).

Nothing built here is committed: the result is a `mpv-dev-x86_64-*.7z` in
the work folder, which takes the place of shinchiro's for the packages.

## Build host

Ubuntu 24.04 (WSL works). Beyond `build-essential`, cmake, ninja, nasm,
yasm, clang, git and curl, the recipe needs:

    sudo apt-get install -y --no-install-recommends mercurial subversion gyp \
        autoconf automake libtool libtool-bin lld libc++1 libc++abi1 libgmp-dev \
        libmpfr-dev libmpc-dev libgcrypt20-dev gperf ragel texinfo autopoint re2c \
        asciidoc docbook2x p7zip-full libmimalloc-dev

and meson and mako from pip (`pip install meson mako jsonschema rst2pdf`,
in a virtual environment whose `bin` is on the PATH). The recipe builds its
own clang toolchain for MinGW and a Rust toolchain in the work folder.

## Size and time

The work folder holds the sources (about 100 projects), the toolchain, the
build and ccache: 30 to 40 GB while it builds. The result is about 30 MB
packed (`libmpv-2.dll` is about 120 MB), as shinchiro's.
