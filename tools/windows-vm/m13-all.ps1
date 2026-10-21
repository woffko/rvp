# M13 checks on the Windows test VM (session 0 over SSH, software OpenGL,
# no sound device, no GPU). Copy this file and run.ps1 to C:\rvp-test with
# the bundle in C:\rvp-test\rvp-bundle (with whisper\ from
# tools/whisper/build.sh), fixture-chapters.mkv and .srt and
# fixture-60s.mkv and fixture-3s.mkv next to rvp.exe, the speech clip
# (speech.wav, made with espeak-ng in WSL) and the tiny model
# (models\ggml-tiny.bin, downloaded by the owner's machine, never
# committed) in C:\rvp-test\m13, and the headless tests built for Windows
# next to the bundle's libmpv-2.dll:
#   cargo xwin test --no-run --release --target x86_64-pc-windows-msvc \
#       -p rvp-app --test subtitles --test speech
#   cargo xwin test --no-run --release --target x86_64-pc-windows-msvc \
#       -p rvp-subtitle --test formats --test editor
# copied as rvp-bundle\subtitles.exe, speech.exe, formats.exe and
# editor.exe.
#
# 1. The M13 self-test (`--selftest=m13 --speech-check`,
#    shell/qml/SelfTestM13.qml): the style keys, the browser and its
#    edits, Input Subtitles, saving, the sync dialog, syncing to the next
#    subtitle, Combine Subtitle, subtitles from the audio with the
#    bundle's whisper.cpp on the CPU, the settings pages.
# 2. whisper.cpp itself: the bundle's whisper-cli on the speech clip with
#    the tiny model (-ng), the text it hears.
# 3. The headless tests: the subtitle files (reader, writer, editor), the
#    subtitle options and RVP's own file work on a headless player, and
#    the subtitles made from the speech clip ahead of time and in real
#    time; RVP_TESTDATA points at the bundle's fixtures.
param(
    [string]$Exe = 'C:\rvp-test\rvp-bundle\rvp.exe',
    [string]$Data = 'C:\rvp-test\m13'
)
$bundle = Split-Path $Exe
$filter = 'DPI|DpiAware|^grabbed=|^qt\.|pw\.conf|ametadata|astats|Parsed_'
$out = 'C:\rvp-test\out\m13'

Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config, $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force C:\rvp-test\config, $out | Out-Null
Set-Content -Path C:\rvp-test\config\settings.json -Value '{ "ipc-server": "" }' -Encoding ascii
$cli = "$bundle\whisper\whisper-cli.exe"
$env:RVP_WHISPER_MODELS = "$Data\models"
Write-Host "environment whisper=$(Test-Path $cli) model=$(Test-Path "$Data\models\ggml-tiny.bin")"

Write-Host '== M13 self-test'
$extra = @('--selftest=m13', '--speech-check', "--out=$out", '--mpv-options=ao=null',
    "$bundle\fixture-chapters.mkv")
& "$PSScriptRoot\run.ps1" -Exe $Exe -Extra $extra | Select-String -NotMatch $filter |
    ForEach-Object { Write-Host $_.Line }
$srt = "$bundle\fixture-chapters.srt"
Write-Host "check fixture_subtitles_unchanged=$(if ((Get-Content -Raw $srt) -match 'First line') { 'PASS' } else { 'FAIL' })"

Write-Host '== whisper.cpp on the speech clip'
if ((Test-Path $cli) -and (Test-Path "$Data\speech.wav") -and (Test-Path "$Data\models\ggml-tiny.bin")) {
    $base = "$out\speech"
    $lines = & $cli -m "$Data\models\ggml-tiny.bin" -f "$Data\speech.wav" -l en -osrt -of $base -ng 2>&1 |
        ForEach-Object { "$_" }
    $lines | Select-String -Pattern 'load_backend|total time' | ForEach-Object { Write-Host $_.Line }
    $text = if (Test-Path "$base.srt") { (Get-Content -Raw "$base.srt").Trim() -replace '\s+', ' ' } else { '' }
    Write-Host "whisper_text=$text"
    Write-Host "check whisper_hears_the_clip=$(if ($text -match '(?i)hello') { 'PASS' } else { 'FAIL' })"
} else {
    Write-Host 'check whisper_hears_the_clip=SKIPPED no whisper-cli, clip or model'
}

$env:RVP_TESTDATA = $bundle
$env:RVP_WHISPER_CLI = $cli
$env:RVP_SPEECH_CLIP = "$Data\speech.wav"
foreach ($test in @('formats', 'editor', 'subtitles', 'speech')) {
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
    $lines | Select-String -Pattern '^test |test result|panicked|assert|^note' |
        ForEach-Object { Write-Host $_.Line }
    Write-Host "check headless_$test=$(if ($code -eq 0) { 'PASS' } else { 'FAIL' })"
}
