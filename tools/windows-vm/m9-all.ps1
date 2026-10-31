# M9 checks on the Windows test VM (session 0 over SSH, software OpenGL,
# no sound device): mpv's on-screen controllers in RVP's window.
#
# Layout: this file and run.ps1 in C:\rvp-test; the bundle in
# C:\rvp-test\rvp-bundle (RVP's libmpv, tools/libmpv/build.sh, with LuaJIT
# and mpv's built-in scripts); in C:\rvp-test\m9 the test file
# fixture-60s.mkv (tools/test-clips) and the users' scripts, each laid out
# as RVP's mpv folder (scripts\, fonts\, script-opts\): uosc\ (uosc's
# release archive unpacked as it is, its bin\ helpers left out) and
# modernz\ (scripts\modernz.lua, fonts\modernz-icons.ttf). The scripts are
# third-party and stay out of the repository.
#
# The M9 self-test (`--selftest=m9`, shell/qml/SelfTestM9.qml) runs three
# times: with mpv's OSC (this libmpv's OSC has menus and RVP's commands on
# its buttons), with uosc and with ModernZ (its seek bar lies higher:
# --controller-bar=0.915). Each run: the controller takes the click on its
# seek bar, RVP the clicks, double click and wheel beside it; mpv's
# fullscreen request moves RVP's window; the window without the skin; the
# controller's menu takes the keys.
param(
    [string]$Exe = 'C:\rvp-test\rvp-bundle\rvp.exe',
    [string]$Data = 'C:\rvp-test\m9'
)
$filter = 'DPI|DpiAware|^grabbed=|^qt\.|pw\.conf|ametadata|astats|Parsed_'
$file = "$Data\fixture-60s.mkv"

function Check([string]$name, [bool]$ok, [string]$detail = '') {
    Write-Host "check $name=$(if ($ok) { 'PASS' } else { 'FAIL' }) $detail"
}

$runs = @(
    @{ Name = 'osc'; Kind = 'osc'; Scripts = ''; Extra = @() },
    @{ Name = 'uosc'; Kind = 'scripts'; Scripts = "$Data\uosc"; Extra = @() },
    @{ Name = 'modernz'; Kind = 'scripts'; Scripts = "$Data\modernz"; Extra = @('--controller-bar=0.915') }
)
foreach ($run in $runs) {
    Write-Host "== the M9 self-test with $($run.Name)"
    if (-not (Test-Path $file)) {
        Check "m9_$($run.Name)" $false "no $file"
        continue
    }
    if ($run.Scripts -ne '' -and -not (Test-Path "$($run.Scripts)\scripts")) {
        Check "m9_$($run.Name)" $false "no scripts in $($run.Scripts)"
        continue
    }
    $out = "C:\rvp-test\out\m9-$($run.Name)"
    Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config, $out -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force C:\rvp-test\config, $out | Out-Null
    Set-Content -Path C:\rvp-test\config\settings.json -Value '{ "ipc-server": "" }' -Encoding ascii
    if ($run.Scripts -ne '') {
        Copy-Item -Recurse $run.Scripts C:\rvp-test\config\mpv
    }
    $extra = @('--selftest=m9', "--controller=$($run.Kind)", "--out=$out", '--skin=', '--mpv-options=ao=null') +
        $run.Extra + @($file)
    $lines = & "$PSScriptRoot\run.ps1" -Exe $Exe -Extra $extra | Select-String -NotMatch $filter |
        ForEach-Object { $_.Line }
    $lines | Where-Object { $_ -match '^(check|failures|exit)' } | ForEach-Object { Write-Host $_ }
    $failures = @($lines | Where-Object { $_ -match '^check .*=FAIL' }).Count
    $ran = @($lines | Where-Object { $_ -match '^check m9_controller_takes_click=PASS' }).Count -eq 1
    Check "m9_$($run.Name)" ($failures -eq 0 -and $ran) "$failures failed"
}
