# M18 checks on the Windows test VM (session 0 over SSH, software OpenGL,
# no sound device, no GPU). Copy this file and run.ps1 to C:\rvp-test with
# the bundle in C:\rvp-test\rvp-bundle, fixture-60s.mkv and
# fixture-square.mkv next to rvp.exe, and the headless tests built for
# Windows next to the bundle's libmpv-2.dll:
#   cargo xwin test --no-run --release --target x86_64-pc-windows-msvc \
#       -p rvp-app --test smoothing
# copied as rvp-bundle\smoothing.exe.
#
# The VapourSynth runtime is the bundle's own (vapoursynth\ next to rvp.exe)
# or, for a bundle without one, -VsScript (the S2 runtime with MVTools,
# through VSSCRIPT_PATH). The VM has no Vulkan: RIFE and fruc_vulkan give
# way to MVTools there, and the host session checks them.
#
# 1. The smoothing self-test (`--selftest=smoothing`, shell/qml/
#    SelfTestSmoothing.qml): Ctrl+Shift+I on and off during playback,
#    MVTools raising the rate, Blend and the presets from the menu's
#    actions, the settings page.
# 2. The GPU power budget's self-test (`--selftest=gpu`) with its
#    smoothing steps: MVTools lighter, Blend, off, and back.
# 3. The headless smoothing tests (`crates/app/tests/smoothing.rs`): the
#    moving square at twice the rate with the square halfway in the frames
#    between, toggles, SVP's filter untouched, the script slot taking
#    over; RVP_TESTDATA points at the bundle's fixtures.
param(
    [string]$Exe = 'C:\rvp-test\rvp-bundle\rvp.exe',
    [string]$VsScript = 'C:\rvp-spikes\s2-bundle\VSScript.dll'
)
$bundle = Split-Path $Exe
$filter = 'DPI|DpiAware|^grabbed=|^qt\.|pw\.conf'
$out = 'C:\rvp-test\out\m18'

function Reset-State {
    Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force C:\rvp-test\config | Out-Null
    Set-Content -Path C:\rvp-test\config\settings.json -Value '{ "ipc-server": "" }' -Encoding ascii
}

Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $out | Out-Null
# The headless tests find the runtime through VSSCRIPT_PATH only.
$env:VSSCRIPT_PATH = if (Test-Path "$bundle\vapoursynth\VSScript.dll") { "$bundle\vapoursynth\VSScript.dll" } else { $VsScript }
Write-Host "environment vsscript=$env:VSSCRIPT_PATH"

foreach ($mode in @('smoothing', 'gpu')) {
    Write-Host "== $mode self-test"
    Reset-State
    $extra = @("--selftest=$mode", "--out=$out\$mode", '--mpv-options=ao=null', "$bundle\fixture-60s.mkv")
    & "$PSScriptRoot\run.ps1" -Exe $Exe -Extra $extra | Select-String -NotMatch $filter |
        ForEach-Object { Write-Host $_.Line }
}

$env:RVP_TESTDATA = $bundle
Write-Host '== headless tests: smoothing'
$binary = "$bundle\smoothing.exe"
if (-not (Test-Path $binary)) {
    Write-Host 'check headless_smoothing=SKIPPED no smoothing.exe'
} else {
    Push-Location $bundle
    $lines = & $binary --test-threads=1 --nocapture 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Pop-Location
    # A panic's message is on the line after "panicked".
    $lines | Select-String -Pattern '^test |test result|panicked|assert|skipped' -Context 0, 1 |
        ForEach-Object {
            Write-Host $_.Line
            if ($_.Line -match 'panicked') { $_.Context.PostContext | ForEach-Object { Write-Host "  $_" } }
        }
    Write-Host "check headless_smoothing=$(if ($code -eq 0) { 'PASS' } else { 'FAIL' })"
}
