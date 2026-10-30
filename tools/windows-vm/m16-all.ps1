# M16 checks on the Windows test VM (session 0 over SSH, software OpenGL,
# no sound device): DVD and Blu-ray menus through RVP's navigators.
#
# Layout: this file and run.ps1 in C:\rvp-test; the bundle in
# C:\rvp-test\rvp-bundle, made with package.py's --discs (libdvdnav-4.dll
# and libbluray-4.dll beside rvp.exe, from tools/discs/build.sh) and RVP's
# libmpv (tools/libmpv/build.sh, without libdvdcss); the synthetic menu
# DVD of tools/test-clips/make-m16-dvd.sh in C:\rvp-test\m16\m16-dvd and
# the synthetic Blu-ray of make-m16-bluray.py in C:\rvp-test\m16\m16-bd; and
# the shell's headless tests built for Windows next to the bundle's
# libmpv-2.dll:
#   cargo xwin test --no-run --release --target x86_64-pc-windows-msvc \
#       -p rvp-shell --bin rvp
# copied as rvp-bundle\shell-tests.exe.
#
# 1. The bundle carries the disc libraries and no libdvdcss, libaacs or
#    keys.
# 2. The M16 self-test (`--selftest=m16`, shell/qml/SelfTestM16.qml) on
#    the synthetic DVD: the main menu with its first button lit, the
#    arrows and the pointer, a click that plays title 1, DVD Control's
#    Audio, Subtitle, Chapter and Root Menu with the machine's streams, a
#    timed still before the menu, and the menus turned off; then on the
#    synthetic Blu-ray: its HDMV menu, the selection, a title, the top
#    menu, the audio stream and the chapter its buttons choose.
# 3. The shell's headless navigation tests on the same discs
#    (shell/src/disc/tests.rs).
param(
    [string]$Exe = 'C:\rvp-test\rvp-bundle\rvp.exe',
    [string]$Disc = 'C:\rvp-test\m16\m16-dvd',
    [string]$Bluray = 'C:\rvp-test\m16\m16-bd'
)
$bundle = Split-Path $Exe
$filter = 'DPI|DpiAware|^grabbed=|^qt\.|pw\.conf|ametadata|astats|Parsed_'

function Check([string]$name, [bool]$ok, [string]$detail = '') {
    Write-Host "check $name=$(if ($ok) { 'PASS' } else { 'FAIL' }) $detail"
}

Write-Host '== 1. the disc libraries in the bundle'
foreach ($name in @('libdvdnav-4.dll', 'libbluray-4.dll')) {
    Check "m16_bundle_has_$name" (Test-Path "$bundle\$name")
}
$forbidden = @(Get-ChildItem -Recurse $bundle -File |
    Where-Object { $_.Name -match 'dvdcss|aacs|KEYDB' } | ForEach-Object { $_.Name })
Check 'm16_bundle_has_no_decryption' ($forbidden.Count -eq 0) ($forbidden -join ' ')

Write-Host '== 2. the M16 self-test'
if (-not (Test-Path "$Disc\VIDEO_TS\VIDEO_TS.IFO")) {
    Check 'm16_selftest' $false "no synthetic disc in $Disc"
} else {
    $out = 'C:\rvp-test\out\m16'
    Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config, $out -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force C:\rvp-test\config, $out | Out-Null
    Set-Content -Path C:\rvp-test\config\settings.json -Value '{ "ipc-server": "" }' -Encoding ascii
    $extra = @('--selftest=m16', "--out=$out", '--mpv-options=ao=null', "--dvd-menus=$Disc", "--bd-menus=$Bluray")
    $lines = & "$PSScriptRoot\run.ps1" -Exe $Exe -Extra $extra | Select-String -NotMatch $filter |
        ForEach-Object { $_.Line }
    $lines | ForEach-Object { Write-Host $_ }
    # The bundle has libdvdnav and libbluray: the checks must run, not be
    # skipped.
    Check 'm16_menus_ran' (@($lines | Where-Object { $_ -match '^check m16_menu_shows=PASS' }).Count -eq 1)
    Check 'm16_bd_menus_ran' (@($lines | Where-Object { $_ -match '^check m16_bd_menu_shows=PASS' }).Count -eq 1)
}

Write-Host '== 3. the headless navigation tests'
$binary = "$bundle\shell-tests.exe"
if (-not (Test-Path $binary)) {
    Write-Host 'check headless_disc=SKIPPED no shell-tests.exe'
} else {
    $env:RVP_DVD_MENU = $Disc
    $env:RVP_BD_MENU = $Bluray
    Push-Location $bundle
    $lines = & $binary synthetic --test-threads=1 --nocapture 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Pop-Location
    $lines | Select-String -Pattern 'test result|panicked|FAILED|note:|libdvdnav|libbluray' | ForEach-Object { Write-Host $_.Line }
    $ran = @($lines | Where-Object { $_ -match '^note: .*skipped$' }).Count -eq 0
    Check 'headless_disc' ($code -eq 0 -and $ran) $(if ($ran) { '' } else { 'skipped' })
}
