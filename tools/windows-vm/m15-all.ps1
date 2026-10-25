# M15 checks on the Windows test VM (session 0 over SSH, software OpenGL,
# no sound device): RVP's control API and its Unified Remote remote.
#
# Layout: this file in C:\rvp-test; the bundle in C:\rvp-test\rvp-bundle
# (with remote\unified-remote\RVP, fixture-60s.mkv beside rvp.exe, and the
# headless tests built for Windows next to its libmpv-2.dll:
#   cargo xwin test --no-run --release --target x86_64-pc-windows-msvc \
#       -p rvp-app --test control --test remote
# copied as control.exe and remote.exe); RVP-*-setup.exe in
# C:\rvp-test\m15; and Unified Remote Server installed on this VM only
# (ServerSetup-3.13.0.2501.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART,
# its SHA-256 checked against winget's manifest).
#
# 1. RVP's installer (silently, for this user) finds Unified Remote and
#    copies the remote into its custom remotes folder, and notes where.
# 2. Unified Remote's server (started here if it does not run: it ends
#    with the SSH session) reloads its remotes and runs the remote's
#    actions for a client of its web-client protocol (/client/connect,
#    /client/request; its manager's /system/remote/run does not run
#    actions in 3.13) while RVP's M15 self-test serves
#    the control API (--remote-serve, before its own calls): the actions
#    reach RVP with the token the remote reads where an installed RVP keeps
#    it (%APPDATA%\RVP\remote-token: the run's state folder is there). The
#    self-test's own checks follow, and the token must not be in its log.
# 3. RVP's uninstaller removes the remote's files, and only those.
# 4. The headless tests: the control API (control.exe) and the remote's
#    files (remote.exe).
param([string]$Exe = 'C:\rvp-test\rvp-bundle\rvp.exe')
$bundle = Split-Path $Exe
$custom = 'C:\ProgramData\Unified Remote\Remotes\Custom'
$remoteDir = "$custom\RVP"
$server = 'C:\Program Files (x86)\Unified Remote 3\RemoteServerWin.exe'
$manager = 'http://localhost:9510'
$filter = 'DPI|DpiAware|^grabbed=|^qt\.|pw\.conf|ametadata|astats|Parsed_'
$remoteFiles = @('icon.png', 'icon_hires.png', 'layout.xml', 'meta.prop', 'remote.lua', 'settings.prop')

function Check([string]$name, [bool]$ok, [string]$detail = '') {
    Write-Host "check $name=$(if ($ok) { 'PASS' } else { 'FAIL' }) $detail"
}
function Manager([string]$path) {
    try {
        (Invoke-WebRequest -UseBasicParsing "$manager/$path" -TimeoutSec 20).Content
    } catch {
        "error: $_"
    }
}
# Unified Remote's web-client protocol (its own /client page speaks it):
# connect, handshake and authenticate (the server asks no password here),
# then each packet runs on that connection. Action 7 runs a remote's action.
$script:urConnection = $null
$script:urSource = 'web-' + [guid]::NewGuid().ToString()
function UrSend($packet) {
    $packet.Source = $script:urSource
    $body = $packet | ConvertTo-Json -Depth 6 -Compress
    try {
        (Invoke-WebRequest -UseBasicParsing -Method Post "$manager/client/request" -TimeoutSec 20 `
            -Headers @{ 'UR-Connection-ID' = $script:urConnection } -Body $body -ContentType 'application/json').Content
    } catch {
        "error: $_"
    }
}
function UrConnect {
    $script:urConnection = ((Manager 'client/connect') | ConvertFrom-Json).id.Trim()
    UrSend @{ Action = 0; Request = 0; Version = 10; Password = [guid]::NewGuid().ToString(); Platform = 'web' } | Out-Null
    UrSend @{ Action = 1; Request = 1
        Capabilities = @{ Actions = $true; Sync = $true; Grid = $true; Fast = $false; Loading = $true; Encryption2 = $true } }
}

if (-not (Test-Path $server)) {
    Write-Host 'check m15_unified_remote=SKIPPED Unified Remote Server is not installed'
    exit 0
}

Write-Host '== 1. the installer copies the remote'
$setup = Get-ChildItem 'C:\rvp-test\m15\RVP-*-setup.exe' | Select-Object -First 1
$installed = "$env:LOCALAPPDATA\Programs\RVP"
Remove-Item -Recurse -Force $remoteDir -ErrorAction SilentlyContinue
Start-Process -Wait -FilePath $setup.FullName -ArgumentList '/S'
$copied = @($remoteFiles | Where-Object { Test-Path "$remoteDir\$_" })
Check 'm15_installer_copies_the_remote' ($copied.Count -eq $remoteFiles.Count) "$($copied.Count) of $($remoteFiles.Count) in $remoteDir"
$noted = (Get-ItemProperty 'HKCU:\Software\RVP' -Name UnifiedRemote -ErrorAction SilentlyContinue).UnifiedRemote
Check 'm15_installer_notes_the_folder' ($noted -eq $remoteDir) "$noted"
$sameLua = (Get-FileHash "$remoteDir\remote.lua").Hash -eq (Get-FileHash "$bundle\remote\unified-remote\RVP\remote.lua").Hash
Check 'm15_installed_remote_is_the_bundles' $sameLua

Write-Host '== 2. Unified Remote runs the remote''s actions'
if (-not (Get-Process RemoteServerWin -ErrorAction SilentlyContinue)) {
    Start-Process $server | Out-Null
    Start-Sleep -Seconds 8
}
$status = Manager 'system/status'
Check 'm15_unified_remote_runs' ($status -match '"running"\s*:\s*true') ($status -split "`n" | Select-String 'version"' | Select-Object -First 1)
Manager 'system/reload' | Out-Null
Start-Sleep -Seconds 3

$appState = "$env:APPDATA\RVP"
$hadState = Test-Path $appState
$out = 'C:\rvp-test\out\m15'
Remove-Item -Recurse -Force C:\rvp-test\config, $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force C:\rvp-test\config, $out | Out-Null
Set-Content -Path C:\rvp-test\config\settings.json -Value '{ "ipc-server": "" }' -Encoding ascii
$env:QT_OPENGL = 'software'
$env:RVP_CONFIG_DIR = 'C:\rvp-test\config'
$env:RVP_STATE_DIR = $appState
$log = "$out\rvp.log"
$actions = @('play_pause', 'volume_down', 'mute', 'mute', 'play_pause')
$rvp = Start-Process -FilePath $Exe -WorkingDirectory $bundle -PassThru -NoNewWindow `
    -RedirectStandardOutput $log -RedirectStandardError "$out\rvp-err.log" -ArgumentList @(
        '--selftest=m15', '--remote-port=48931', '--remote-serve=90', "--remote-expect=$($actions.Count)",
        "--out=$out", '--mpv-options=ao=null', "`"$bundle\fixture-60s.mkv`"")
# Read now, or the exit code is lost.
$null = $rvp.Handle
# The self-test serves first: wait until the API answers with the token
# (the redirected log cannot be read while RVP writes it).
$tokenFile = "$appState\remote-token"
$deadline = (Get-Date).AddSeconds(60)
$up = $false
while (-not $up -and (Get-Date) -lt $deadline -and -not $rvp.HasExited) {
    Start-Sleep -Milliseconds 500
    if (-not (Test-Path $tokenFile)) { continue }
    $headers = @{ Authorization = 'Bearer ' + (Get-Content $tokenFile -Raw).Trim() }
    try {
        $up = (Invoke-WebRequest -UseBasicParsing 'http://127.0.0.1:48931/api/status' -Headers $headers -TimeoutSec 5).StatusCode -eq 200
    } catch {
        $up = $false
    }
}
Check 'm15_api_answers_the_script' $up
Start-Sleep -Seconds 2
$authenticated = UrConnect
Check 'm15_unified_remote_client_connects' ($authenticated -match '"Security"\s*:\s*1') (($authenticated -replace '\s+', ' ').Trim())
foreach ($action in $actions) {
    $answer = UrSend @{ ID = 'RVP.Player'; Action = 7; Request = 7; Run = @{ Name = $action } }
    Write-Host "unified_remote_run $action -> $(($answer -replace '\s+', ' ').Trim())"
    Start-Sleep -Milliseconds 700
}
# The remote stops polling RVP.
UrSend @{ ID = 'RVP.Player'; Action = 4; Request = 4 } | Out-Null
Manager 'client/disconnect' | Out-Null
if (-not $rvp.WaitForExit(120000)) {
    Stop-Process -Id $rvp.Id -Force
    Write-Host 'check m15_selftest_ends=FAIL killed'
}
Get-Content $log, "$out\rvp-err.log" -ErrorAction SilentlyContinue | Select-String -NotMatch $filter |
    Where-Object { $_.Line -match '^check |^failures=|m15_serving' } | ForEach-Object { Write-Host $_.Line }
Write-Host "rvp_exit=$($rvp.ExitCode)"
if (Test-Path $tokenFile) {
    $token = (Get-Content $tokenFile -Raw).Trim()
    $leaked = Select-String -Path $log, "$out\rvp-err.log" -SimpleMatch $token -Quiet -ErrorAction SilentlyContinue
    Check 'm15_token_not_logged' (-not $leaked)
}
$urLog = Manager 'system/log/6/6'
$urLines = @($urLog -split '"' | Where-Object { $_ -match 'RVP' })
$urLines | Select-Object -Last 12 | ForEach-Object { Write-Host "unified_remote_log $_" }
$urErrors = @($urLines | Where-Object { $_ -match 'error|fail' })
Check 'm15_unified_remote_log_clean' ($urErrors.Count -eq 0) ($urErrors -join ' | ')
if (-not $hadState) {
    Remove-Item -Recurse -Force $appState -ErrorAction SilentlyContinue
}

Write-Host '== 3. the uninstaller removes the remote'
Start-Process -Wait -FilePath "$installed\uninstall.exe" -ArgumentList '/S'
Start-Sleep -Seconds 2
Check 'm15_uninstaller_removes_the_remote' (-not (Test-Path $remoteDir)) "$remoteDir"
Check 'm15_custom_folder_kept' (Test-Path $custom) "$custom"
Remove-Item -Recurse -Force $installed -ErrorAction SilentlyContinue

foreach ($test in @('control', 'remote')) {
    Write-Host "== 4. headless tests: $test"
    $binary = "$bundle\$test.exe"
    if (-not (Test-Path $binary)) {
        Write-Host "check headless_$test=SKIPPED no $test.exe"
        continue
    }
    Push-Location $bundle
    $env:RVP_TESTDATA = $bundle
    $env:RVP_REMOTE_DIR = "$bundle\remote\unified-remote\RVP"
    $lines = & $binary --test-threads=1 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Pop-Location
    $lines | Select-String -Pattern 'test result|panicked|FAILED' | ForEach-Object { Write-Host $_.Line }
    Check "headless_$test" ($code -eq 0)
}
