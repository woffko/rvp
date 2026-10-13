# M6 checks on the Windows test VM (session 0 over SSH, software OpenGL):
# the functional self-test with M6's checks (the control panel, a colour and
# an equalizer change, a shader toggle, SVP's diagnostic), then the
# orientation check, which is SKIPPED here: session-0 grabs show no video.
#
# Layout as m4-all.ps1, plus fixture-marker.mkv and fixture-marker.png in
# the bundle. SVP mode is off ("ipc-server": "") as in the kit's runs.
$common = @('--out=C:\rvp-test\out', '--skin=C:\rvp-test\BlackSpace_en.dsf', '--mpv-options=ao=null')
$bundle = 'C:\rvp-test\rvp-bundle'
$long = "$bundle\fixture-60s.mkv"
$short = "$bundle\fixture-3s.mkv"
$filter = 'DPI|DpiAware|^grabbed=|^qt\.'
Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config, C:\rvp-test\out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force C:\rvp-test\config | Out-Null
Set-Content -Path C:\rvp-test\config\settings.json -Value '{ "ipc-server": "" }' -Encoding ascii

Write-Host '== functional (M6 checks)'
$lines = & "$PSScriptRoot\run.ps1" -Extra (@('--selftest') + $common + @($long, $short)) |
    Select-String -NotMatch $filter | ForEach-Object { $_.Line }
$lines | ForEach-Object { Write-Host $_ }
$names = 'f7_opens_panel', 'color_slider_applies', 'color_reset_applies', 'equalizer_slider_applies',
    'equalizer_button_toggles', 'f7_closes_panel', 'shift_p_shaders_on', 'shift_p_shaders_off',
    'svp_diagnostic_shows', 'svp_mode_off'
$missing = @($names | Where-Object { -not ($lines -match "^check $_=PASS") })
Write-Host "m6_missing=$($missing.Count) $($missing -join ',')"

Write-Host '== upright'
Remove-Item -Recurse -Force C:\rvp-test\state -ErrorAction SilentlyContinue
Set-Content -Path C:\rvp-test\config\settings.json -Value '{ "ipc-server": "", "fit-window-to-video": false }' -Encoding ascii
& "$PSScriptRoot\run.ps1" -Extra (@('--selftest=upright') + $common + @("$bundle\fixture-marker.mkv")) |
    Select-String -NotMatch $filter | Select-String -Pattern '^(check |failures=|exit=)' |
    ForEach-Object { Write-Host $_.Line }
