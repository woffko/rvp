# M10 checks on the Windows test VM (session 0 over SSH, software OpenGL,
# no sound device). Copy this file and run.ps1 to C:\rvp-test with the
# bundle in C:\rvp-test\rvp-bundle (and the fixtures, as for m7-all.ps1),
# and the level tests next to the bundle's libmpv-2.dll:
#   cargo xwin test --no-run --release --target x86_64-pc-windows-msvc \
#       -p rvp-app --test audio_chain --test speakers
# copied as rvp-bundle\audio_chain.exe and rvp-bundle\speakers.exe.
#
# 1. The functional self-test on RVP Default, with M10's steps: effects in
#    the chain, PotPlayer's 26 speaker choices, the individual volume,
#    Freeverb's response, the sync dialog, the second device (its core on
#    ao=null as well), recording, the system's master volume and Audio
#    Stretching (Shift+J, Shift+K).
# 2. Kept awake: while the self-test plays video, `powercfg /requests`
#    lists rvp.exe, and not after it. Session 0 has no display, so Windows
#    lists the request under SYSTEM only (a probe asking for both from
#    PowerShell showed the same); DISPLAY is printed for the record.
# 3. The recording's file: a RIFF WAVE whose sizes match its length (the
#    VM has no device, so it may hold no samples).
# 4. The audio chain's level tests and the speakers' mix on Windows' own
#    FFmpeg (the bundle's libmpv-2.dll).
param([string]$Exe = 'C:\rvp-test\rvp-bundle\rvp.exe')
$bundle = Split-Path $Exe
$long = "$bundle\fixture-60s.mkv"
$short = "$bundle\fixture-3s.mkv"
$filter = 'DPI|DpiAware|^grabbed=|^qt\.'
$out = 'C:\rvp-test\out\m10'

# The sections of `powercfg /requests` that list rvp.exe.
function Get-RvpRequests {
    $section = ''
    $found = @()
    foreach ($line in (powercfg /requests 2>&1 | ForEach-Object { "$_" })) {
        if ($line -match '^([A-Z]+):\s*$') { $section = $Matches[1]; continue }
        if ($line -match 'rvp\.exe' -and $found -notcontains $section) { $found += $section }
    }
    return $found
}

Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config, $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force C:\rvp-test\config, $out | Out-Null
Set-Content -Path C:\rvp-test\config\settings.json -Value '{ "ipc-server": "" }' -Encoding ascii
Write-Host "requests_before=$((Get-RvpRequests) -join ',')"

Write-Host '== functional (RVP Default, M10 steps)'
$env:QT_OPENGL = 'software'
$env:RVP_CONFIG_DIR = 'C:\rvp-test\config'
$env:RVP_STATE_DIR = 'C:\rvp-test\state'
$arguments = @('--selftest', "--out=$out", '--mpv-options=ao=null', $long, $short)
$process = Start-Process -FilePath $Exe -ArgumentList $arguments -WorkingDirectory $bundle `
    -RedirectStandardOutput "$out\stdout.txt" -RedirectStandardError "$out\stderr.txt" `
    -NoNewWindow -PassThru
# Without the handle cached, ExitCode stays empty.
$null = $process.Handle
$seen = @()
while (-not $process.HasExited) {
    foreach ($section in (Get-RvpRequests)) {
        if ($seen -notcontains $section) { $seen += $section }
    }
    Start-Sleep -Milliseconds 1500
}
$process.WaitForExit()
Get-Content "$out\stdout.txt", "$out\stderr.txt" -ErrorAction SilentlyContinue |
    Select-String -NotMatch $filter | ForEach-Object { Write-Host $_.Line }
Write-Host "exit=$($process.ExitCode)"
Write-Host "requests_while_playing=$($seen -join ',')"
Write-Host "check kept_awake_while_playing=$(if ($seen -contains 'SYSTEM') { 'PASS' } else { 'FAIL' })"
$after = Get-RvpRequests
Write-Host "check awake_request_released=$(if ($after.Count -eq 0) { 'PASS' } else { 'FAIL' })"

Write-Host '== recording file'
$wav = "$out\recording.wav"
if (Test-Path $wav) {
    $bytes = [System.IO.File]::ReadAllBytes($wav)
    $riff = [System.Text.Encoding]::ASCII.GetString($bytes, 0, 4)
    $wave = [System.Text.Encoding]::ASCII.GetString($bytes, 8, 4)
    $riffSize = [BitConverter]::ToUInt32($bytes, 4)
    # Chunks after the 12-byte header: find "data".
    $at = 12
    $dataSize = -1
    $dataAt = -1
    while ($at + 8 -le $bytes.Length) {
        $id = [System.Text.Encoding]::ASCII.GetString($bytes, $at, 4)
        $size = [BitConverter]::ToUInt32($bytes, $at + 4)
        if ($id -eq 'data') { $dataSize = $size; $dataAt = $at + 8; break }
        $at += 8 + $size + ($size % 2)
    }
    $ok = ($riff -eq 'RIFF') -and ($wave -eq 'WAVE') -and ($riffSize -eq $bytes.Length - 8) `
        -and ($dataAt -ge 0) -and ($dataAt + $dataSize -eq $bytes.Length)
    Write-Host "recording_bytes=$($bytes.Length) data=$dataSize"
    Write-Host "check recording_file_sizes=$(if ($ok) { 'PASS' } else { 'FAIL' })"
} else {
    Write-Host 'check recording_file_sizes=SKIPPED no recording (see recording_starts)'
}

foreach ($test in @('audio_chain', 'speakers')) {
    Write-Host "== level tests: $test"
    $binary = "$bundle\$test.exe"
    if (-not (Test-Path $binary)) {
        Write-Host "check level_$test=SKIPPED no $test.exe"
        continue
    }
    Push-Location $bundle
    $lines = & $binary --test-threads=2 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Pop-Location
    $lines | Select-String -Pattern '^test |test result|panicked|assert' | ForEach-Object { Write-Host $_.Line }
    Write-Host "check level_$test=$(if ($code -eq 0) { 'PASS' } else { 'FAIL' })"
}
