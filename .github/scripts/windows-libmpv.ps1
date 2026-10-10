# Fetches libmpv for the Windows CI job (the shinchiro build used since goal B)
# and makes an MSVC import library from the DLL's exports. Sets MPV_LIB_DIR
# for the linker and adds the DLL's folder to PATH for the tests.
$ErrorActionPreference = 'Stop'
$tag = '20261010'
$asset = 'mpv-dev-x86_64-20261010-git-b2c255c13e.7z'
$dir = Join-Path $env:RUNNER_TEMP 'libmpv'
New-Item -ItemType Directory -Force $dir | Out-Null
$archive = Join-Path $dir $asset
if (-not (Test-Path $archive)) {
    gh release download $tag -R shinchiro/mpv-winbuild-cmake -p $asset -D $dir --clobber
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
