# RVP's libmpv for Windows

RVP's Windows packages used shinchiro's libmpv builds
(`mpv-dev-x86_64-*-git-*.7z` from shinchiro/mpv-winbuild-cmake's releases).
Those builds have FFmpeg without a SPIR-V compiler, so FFmpeg leaves out
its Vulkan compute filters, and with them `fruc_vulkan` (NVIDIA's optical
flow), M18's fourth smoothing engine (D18). FFmpeg compiles its Vulkan
shaders while it builds, with a `glslc`, `glslang` or `glslangValidator` its
configure finds on the PATH (it has no `--enable-libshaderc` any more), and
shinchiro's build host has none. `build.sh` builds libmpv with the same
recipe at a fixed commit, after it builds glslang 16.6.0 for the build host
into the work folder and puts it on the PATH.

Four changes to the recipe:

- `curl-release.patch` holds curl at its 8.22.0 release: curl's master
  (since 8.23.0) calls OpenSSL 4.1's `ASN1_STRING_get_length`, which the
  recipe's pinned OpenSSL (4.1.0-dev of 2026-07-18) does not have yet.
- `curl-no-libssh.patch` builds curl without SSH: libssh's master
  removed the SCP API that curl 8.22.0 still uses, and a player needs no
  SCP or SFTP (goal R).
- `ffmpeg-no-opengl.patch` drops `--enable-opengl`: FFmpeg's master
  removed its OpenGL output device, which RVP never used (goal R).
- `no-libdvdcss.patch` builds libdvdread without libdvdcss (goal R): RVP
  never ships libdvdcss, and libdvdread loads a user's `libdvdcss-2.dll`
  at run time instead. `build.sh` fails when the result still holds
  libdvdcss's strings.

The recipe takes every other package at its newest commit, so a later
build may need another pin like these.

Nothing built here is committed: the result is a `mpv-dev-x86_64-*.7z` in
the work folder, which takes the place of shinchiro's for the packages
and, since 2026-11-21, for CI (see "The published archive").

## Build host

Ubuntu 24.04 (WSL works). Beyond `build-essential`, cmake, ninja, nasm,
yasm, clang, git and curl, the recipe needs:

    sudo apt-get install -y --no-install-recommends mercurial subversion gyp \
        autoconf automake libtool libtool-bin lld libc++-dev libc++abi-dev libgmp-dev \
        libmpfr-dev libmpc-dev libgcrypt20-dev gperf ragel texinfo autopoint re2c \
        asciidoc docbook2x p7zip-full

libc++'s headers (`libc++-dev`, `libc++abi-dev`), not only its libraries
as the recipe's own README lists for Ubuntu: the recipe builds its LLVM
with libc++ and `-ftls-model=local-exec`, and with libstdc++ instead the
link of `llvm-profdata` fails (`R_X86_64_TPOFF32` against
`std::__once_callable`).

and meson and mako from pip (`pip install meson mako jsonschema rst2pdf`,
in a virtual environment whose `bin` is on the PATH). The recipe builds its
own clang toolchain for MinGW and a Rust toolchain in the work folder.

Without sudo, `rootless.sh` does the same in a build root of its own: it
unpacks Ubuntu's 24.04 base image (checked against Ubuntu's signed
checksums) into `~/rvp-deps/mpv-root`, enters it as root of a user
namespace that maps only the caller's user, installs the packages and
meson there, and runs `build.sh` inside with the work folder and the
checkout (read-only) mounted at their own paths. The system's packages
stay as they are. RVP's libmpv of 2026-10-19 and of 2026-10-29 were built
this way.

## The published archive

CI and the releases' Windows packages take the same libmpv: the newest
archive `build.sh` made, as `package.py libmpv` prepares it:

    python3 tools/packaging/package.py libmpv \
        --archive WORK/build64/mpv-dev-x86_64-DATE-git-HASH.7z --out OUT --sevenzip 7z

It refuses an archive with libdvdcss inside, gives the build host's paths
inside `libmpv-2.dll` a neutral name of the same length (they name the
builder's account; nothing else in the file changes but its checksum),
packs the archive again as `rvp-mpv-dev-...7z` and prints its SHA-256.
That file, the commits of its sources (`build.sh`'s `...-sources.txt`)
and the licence files of what it holds (`licences.sh`'s folder, packed)
are the files of the repository's release `libmpv-DATE`: a pre-release
that is a build input, not a download for users (D22).
`.github/scripts/windows-libmpv.ps1` names the release, the file and its
SHA-256. After a new build, publish the new archive first, change those
three there and the caches' key in `.github/workflows/ci.yml`, and build
the packages with the DLL of the published archive, so CI tests the
library a release carries.

## Size and time

The work folder holds the sources (about 100 projects), the toolchain, the
build and ccache: 30 to 40 GB while it builds. The result is about 30 MB
packed (`libmpv-2.dll` is about 120 MB), as shinchiro's.
