# M14 checks on the Windows test VM (session 0 over SSH, software OpenGL,
# no sound device, no GPU). Copy this file and run.ps1 to C:\rvp-test with
# the bundle in C:\rvp-test\rvp-bundle, fixture-60s.mkv and
# fixture-tone.ogg next to rvp.exe, the test copy of BlackSpace in
# C:\rvp-test\BlackSpace_en.dsf, and the headless tests built for Windows
# next to the bundle's libmpv-2.dll:
#   cargo xwin test --no-run --release --target x86_64-pc-windows-msvc \
#       -p rvp-app --test settings --test translations --test menus
#   cargo xwin test --no-run --release --target x86_64-pc-windows-msvc \
#       -p rvp-skin --test synthetic
# copied as rvp-bundle\settings.exe, translations.exe, menus.exe and
# synthetic.exe.
#
# 1. The M14 self-test (`--selftest=m14 --registry-check`,
#    shell/qml/SelfTestM14.qml) with RVP Default and with BlackSpace: skin
#    switching, colour themes, opacity, the audio skin, auto hide and the
#    on-screen control, copied times, menu files and styles, OSD and info
#    items, Playback/System Info, presets, the Accessibility options, the
#    interface in Russian, the settings window's search and Reset Page,
#    the playlist's thumbnail images, and on Windows the Run key and the
#    file types in the current user's registry (each put back).
# 2. The registry after the runs: no Run value, and the RVP.* types the
#    VM had before (an installed copy's) and no more.
# 3. The headless tests: settings and presets, the translations, the menus
#    and the synthetic skin.
param(
    [string]$Exe = 'C:\rvp-test\rvp-bundle\rvp.exe',
    [string]$Skin = 'C:\rvp-test\BlackSpace_en.dsf'
)
$bundle = Split-Path $Exe
$filter = 'DPI|DpiAware|^grabbed=|^qt\.|pw\.conf|ametadata|astats|Parsed_'

function Get-RvpTypes {
    @(Get-ChildItem 'HKCU:\Software\Classes' -ErrorAction SilentlyContinue |
        Where-Object { $_.PSChildName -like 'RVP.*' }).Count
}
$typesBefore = Get-RvpTypes

$runs = @(
    @{ Name = 'default'; Skin = @() },
    @{ Name = 'blackspace'; Skin = @("--skin=$Skin") }
)
foreach ($run in $runs) {
    $out = "C:\rvp-test\out\m14-$($run.Name)"
    Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config, $out -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force C:\rvp-test\config, $out | Out-Null
    Set-Content -Path C:\rvp-test\config\settings.json -Value '{ "ipc-server": "" }' -Encoding ascii
    if ($run.Skin.Count -gt 0 -and -not (Test-Path $Skin)) {
        Write-Host "check m14_$($run.Name)=SKIPPED no $Skin"
        continue
    }
    Write-Host "== M14 self-test: $($run.Name)"
    $extra = @('--selftest=m14', '--registry-check', "--out=$out", '--mpv-options=ao=null') + $run.Skin +
        @("$bundle\fixture-60s.mkv", "$bundle\fixture-tone.ogg")
    & "$PSScriptRoot\run.ps1" -Exe $Exe -Extra $extra | Select-String -NotMatch $filter |
        ForEach-Object { Write-Host $_.Line }
}

Write-Host '== the registry after the runs'
$runValue = Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name RVP `
    -ErrorAction SilentlyContinue
Write-Host "check run_key_left_clean=$(if ($null -eq $runValue) { 'PASS' } else { 'FAIL' })"
$typesAfter = Get-RvpTypes
Write-Host "check types_left_clean=$(if ($typesAfter -eq $typesBefore) { 'PASS' } else { 'FAIL' }) before $typesBefore after $typesAfter"

$env:RVP_TEST_SKIN = $Skin
foreach ($test in @('settings', 'translations', 'menus', 'synthetic')) {
    Write-Host "== headless tests: $test"
    $binary = "$bundle\$test.exe"
    if (-not (Test-Path $binary)) {
        Write-Host "check headless_$test=SKIPPED no $test.exe"
        continue
    }
    Push-Location $bundle
    $lines = & $binary --test-threads=1 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Pop-Location
    $lines | Select-String -Pattern 'test result|panicked|FAILED' | ForEach-Object { Write-Host $_.Line }
    Write-Host "check headless_$test=$(if ($code -eq 0) { 'PASS' } else { 'FAIL' })"
}
