# Fetches libmpv for the Windows CI job and makes an MSVC import library
# from the DLL's exports. Sets MPV_LIB_DIR for the linker and adds the DLL's
# folder to PATH for the tests.
#
# The libmpv is RVP's own build (tools/libmpv/build.sh: no libdvdcss, with
# FFmpeg's Vulkan filters), the one the releases' Windows packages carry,
# so CI tests what is shipped and its packages hold nothing a release may
# not. It is a file of this repository's release `libmpv-<date>`
# (tools/libmpv/README.md, "The published archive"; D22). Until 2026-11-21
# CI took shinchiro's build, which has libdvdcss inside.
$ErrorActionPreference = 'Stop'
$tag = 'libmpv-20261029'
$asset = 'rvp-mpv-dev-x86_64-20261029-git-6ee40e64d3.7z'
# The archive's SHA-256 (`package.py libmpv` prints it): checked after a
# download and on a cache hit alike.
$sha256 = '23a407f24c4e3072af86e1fffb6c9e65b0363bbc15229dbfe772e84ee0dba6cb'
$dir = Join-Path $env:RUNNER_TEMP 'libmpv'
New-Item -ItemType Directory -Force $dir | Out-Null
$archive = Join-Path $dir $asset
if (-not (Test-Path $archive)) {
    gh release download $tag -R woffko/rvp -p $asset -D $dir --clobber
}
$found = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
if ($found -ne $sha256) {
    Remove-Item -Force $archive
    throw "$asset has SHA-256 $found, expected $sha256"
}
7z x -y "-o$dir" $archive | Out-Null
$dll = Join-Path $dir 'libmpv-2.dll'
$llvm = 'C:\Program Files\LLVM\bin'
$names = & "$llvm\llvm-readobj.exe" --coff-exports $dll |
    Select-String '^\s*Name: ' |
    ForEach-Object { '    ' + ($_.Line -split 'Name: ')[1].Trim() }
$def = Join-Path $dir 'mpv.def'
Set-Content -Encoding ascii $def (@('LIBRARY libmpv-2.dll', 'EXPORTS') + $names)
& "$llvm\llvm-dlltool.exe" -m i386:x86-64 -d $def -D libmpv-2.dll -l (Join-Path $dir 'mpv.lib')
"MPV_LIB_DIR=$dir" | Out-File -Append -Encoding utf8 $env:GITHUB_ENV
$dir | Out-File -Append -Encoding utf8 $env:GITHUB_PATH
Write-Output "libmpv in $dir ($($names.Count) exports)"
