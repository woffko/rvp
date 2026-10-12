# M4 and M5 checks on the Windows test VM (session 0 over SSH, software
# OpenGL): the functional self-test with its menu and settings checks, then
# portable mode (rvp.portable next to rvp.exe keeps settings.json and the
# state next to it). The marker and the portable files are removed at the
# end.
#
# Layout as m3-all.ps1: C:\rvp-test\rvp-bundle (rvp.exe, Qt, libmpv,
# fixture-*.mkv), C:\rvp-test\BlackSpace_en.dsf, this script and run.ps1
# next to it.
$common = @('--out=C:\rvp-test\out', '--skin=C:\rvp-test\BlackSpace_en.dsf', '--mpv-options=ao=null')
$long = 'C:\rvp-test\rvp-bundle\fixture-60s.mkv'
$short = 'C:\rvp-test\rvp-bundle\fixture-3s.mkv'
$bundle = 'C:\rvp-test\rvp-bundle'
Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config, C:\rvp-test\out -ErrorAction SilentlyContinue

Write-Host '== functional (M4 and M5 checks)'
$lines = & "$PSScriptRoot\run.ps1" -Extra (@('--selftest') + $common + @($long, $short)) |
    Select-String -NotMatch 'DPI|DpiAware|^grabbed=|^qt\.' | ForEach-Object { $_.Line }
$lines | ForEach-Object { Write-Host $_ }
$names = 'context_menu_keys', 'context_menu_marks', 'playlist_menu_keys', 'context_menu_builds',
    'menu_command_runs', 'f5_opens_settings', 'settings_pages_load', 'keys_page_rows',
    'setting_applies_at_once', 'settings_window_closes'
$missing = @($names | Where-Object { -not ($lines -match "^check $_=PASS") })
Write-Host "m4_missing=$($missing.Count) $($missing -join ',')"

Write-Host '== portable'
Remove-Item -Recurse -Force "$bundle\settings.json", "$bundle\state" -ErrorAction SilentlyContinue
New-Item -ItemType File -Force "$bundle\rvp.portable" | Out-Null
Set-Location $bundle
$env:QT_OPENGL = 'software'
Remove-Item Env:RVP_CONFIG_DIR, Env:RVP_STATE_DIR -ErrorAction SilentlyContinue
& .\rvp.exe --selftest=playback --play-seconds=3 @common $long 2>&1 |
    ForEach-Object { "$_" } | Select-String -Pattern '^(check |failures=)' | ForEach-Object { Write-Host $_.Line }
Write-Host "exit=$LASTEXITCODE"
$portable = (Test-Path "$bundle\settings.json") -and (Test-Path "$bundle\state")
Write-Host "portable=$portable"
Remove-Item -Recurse -Force "$bundle\rvp.portable", "$bundle\settings.json", "$bundle\state" -ErrorAction SilentlyContinue
