# M7 checks on the Windows test VM (session 0 over SSH, software OpenGL).
# PotPlayer is not installed there, so with no skin set RVP starts with
# RVP Default, its own skin, and the functional self-test (with the control
# panel's checks) runs on it. A second run with the copy of BlackSpace kept
# for tests (C:\rvp-test\BlackSpace_en.dsf) shows RVP's logo in that skin's
# title bar (decision D10); the window grabs go to C:\rvp-test\out\<run>.
# Last, the icon Explorer shows for rvp.exe.
#
# Layout as m6-all.ps1 (copy this file and run.ps1 to C:\rvp-test with the
# bundle in C:\rvp-test\rvp-bundle). SVP mode is off ("ipc-server": "").
# `-Exe` runs another copy with the bundle's fixtures (m8-all.ps1: the
# installed one).
param([string]$Exe = 'C:\rvp-test\rvp-bundle\rvp.exe')
$bundle = 'C:\rvp-test\rvp-bundle'
$long = "$bundle\fixture-60s.mkv"
$short = "$bundle\fixture-3s.mkv"
$filter = 'DPI|DpiAware|^grabbed=|^qt\.'
$installed = (Test-Path 'C:\Program Files\DAUM\PotPlayer') -or (Test-Path 'C:\Program Files (x86)\DAUM\PotPlayer')
Write-Host "potplayer_installed=$installed"

$runs = @(
    @{ Name = 'default'; Skin = @() },
    @{ Name = 'blackspace'; Skin = @('--skin=C:\rvp-test\BlackSpace_en.dsf') }
)
foreach ($run in $runs) {
    $out = "C:\rvp-test\out\$($run.Name)"
    Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config, $out -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force C:\rvp-test\config, $out | Out-Null
    Set-Content -Path C:\rvp-test\config\settings.json -Value '{ "ipc-server": "" }' -Encoding ascii
    Write-Host "== functional ($($run.Name))"
    $extra = @('--selftest', "--out=$out", '--mpv-options=ao=null') + $run.Skin + @($long, $short)
    & "$PSScriptRoot\run.ps1" -Exe $Exe -Extra $extra | Select-String -NotMatch $filter |
        ForEach-Object { Write-Host $_.Line }
}

Write-Host '== exe icon'
Add-Type -AssemblyName System.Drawing
$icon = [System.Drawing.Icon]::ExtractAssociatedIcon($Exe)
$icon.ToBitmap().Save('C:\rvp-test\out\exe-icon.png')
Write-Host "exe_icon=$($icon.Width)x$($icon.Height)"
