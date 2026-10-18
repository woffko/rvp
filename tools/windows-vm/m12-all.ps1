# M12 checks on the Windows test VM (session 0 over SSH, software OpenGL,
# no sound device). Copy this file and run.ps1 to C:\rvp-test with the
# bundle in C:\rvp-test\rvp-bundle and M12's fixtures next to rvp.exe
# (testdata/media: fixture-60s.mkv, fixture-3s.mkv, fixture-marker.mkv,
# fixture-marker.png, fixture-sbs.mkv, fixture-360.mkv, fixture-tone.ogg,
# fixture-chapters.mkv and fixture-chapters.srt),
# and the headless tests built for Windows next to the bundle's
# libmpv-2.dll:
#   cargo xwin test --no-run --release --target x86_64-pc-windows-msvc \
#       -p rvp-app --test m12
#   cargo xwin test --no-run --release --target x86_64-pc-windows-msvc \
#       -p rvp-engine --test headless
# copied as rvp-bundle\m12.exe and rvp-bundle\headless.exe.
#
# 1. The M12 self-test (`--selftest=m12`, shell/qml/SelfTestM12.qml): the
#    window's sizes, dialogs, moves and fullscreen on another display,
#    capture (BMP, the screen frame, the clipboard, consecutive images, the
#    thumbnail image) and the settings window's tree.
# 2. The shader check (`--selftest=shaders`): RVP's own shaders, and
#    PotPlayer's when -PotPlayerShaders names a folder of them (read in
#    place), compiled by mpv; mpv's log goes to the out folder. The VM's
#    software OpenGL (Mesa 11, OpenGL 3.0) can neither create the textures
#    mpv's hooks render into nor compile their GLSL: there each shader
#    reports SKIPPED, and the shaders are checked under Linux's Mesa and
#    on the host.
# 3. M12's headless tests on Windows' FFmpeg (3D and 360° pixel checks,
#    effects, capture formats, recordings and the second core's encoding,
#    visualizations, rendering and decoder options), with RVP_TESTDATA
#    pointing at the bundle's fixtures.
# 4. What Windows' libmpv offers: its FFmpeg's AviSynth and VapourSynth
#    demuxers (opening .avs and .vpy files) and its encoders.
param(
    [string]$Exe = 'C:\rvp-test\rvp-bundle\rvp.exe',
    [string]$PotPlayerShaders = ''
)
$bundle = Split-Path $Exe
$filter = 'DPI|DpiAware|^grabbed=|^qt\.'
$out = 'C:\rvp-test\out\m12'

function Reset-State {
    Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force C:\rvp-test\config | Out-Null
}

Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $out | Out-Null

Write-Host '== M12 self-test'
Reset-State
Set-Content -Path C:\rvp-test\config\settings.json -Value '{ "ipc-server": "" }' -Encoding ascii
$extra = @('--selftest=m12', "--out=$out", '--mpv-options=ao=null', "$bundle\fixture-60s.mkv")
& "$PSScriptRoot\run.ps1" -Exe $Exe -Extra $extra | Select-String -NotMatch $filter |
    ForEach-Object { Write-Host $_.Line }

Write-Host '== shaders'
Reset-State
$folders = if ($PotPlayerShaders -ne '') { '"' + ($PotPlayerShaders -replace '\\', '\\') + '"' } else { '' }
Set-Content -Path C:\rvp-test\config\settings.json `
    -Value "{ `"ipc-server`": `"`", `"shaders`": { `"folders`": [$folders] } }" -Encoding ascii
$log = "$out\mpv-shaders.log"
$extra = @('--selftest=shaders', "--out=$out", "--mpv-log=$log",
    "--mpv-options=ao=null,log-file=$log", "$bundle\fixture-60s.mkv")
& "$PSScriptRoot\run.ps1" -Exe $Exe -Extra $extra | Select-String -NotMatch $filter |
    ForEach-Object { Write-Host $_.Line }

$env:RVP_TESTDATA = $bundle
foreach ($test in @('m12', 'headless')) {
    Write-Host "== headless tests: $test"
    $binary = "$bundle\$test.exe"
    if (-not (Test-Path $binary)) {
        Write-Host "check headless_$test=SKIPPED no $test.exe"
        continue
    }
    Push-Location $bundle
    $lines = & $binary --test-threads=1 --nocapture 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Pop-Location
    # A panic's message is on the line after "panicked".
    $lines | Select-String -Pattern '^test |test result|panicked|assert|^metric|^note' -Context 0, 1 |
        ForEach-Object {
            Write-Host $_.Line
            if ($_.Line -match 'panicked') { $_.Context.PostContext | ForEach-Object { Write-Host "  $_" } }
        }
    Write-Host "check headless_$test=$(if ($code -eq 0) { 'PASS' } else { 'FAIL' })"
}
