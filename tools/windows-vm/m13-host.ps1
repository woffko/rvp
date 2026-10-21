# M13's host session (goal O): whisper.cpp on the owner's RTX card and in
# RVP's own window, run only with the owner's permission after advance
# notice, every time. A UAC window asks once for the power limit; the
# whisper.cpp runs start no window of their own (this console shows their
# lines); RVP's window appears once, for about a minute, with mpv's sound
# off.
#
# -Bundle is a portable RVP with whisper\ (tools/packaging/package.py
# --whisper); -Data holds speech-long.wav (tools/test-clips/
# make-m13-speech.sh) and models\ with ggml-base.bin and ggml-small.bin
# (whisper.cpp's models, downloaded, never committed). gpu-limit.ps1 must
# be next to this script.
#
# Every GPU run goes at -PowerLimit percent (default 90) of the card's
# default power limit through gpu-limit.ps1 (D17), which puts the limit it
# found back when this script ends, and is guarded as in m18-host.ps1:
# nvidia-smi must see the card before and after each run, and a lost GPU,
# a Vulkan device error or a limit that went back up stops the script at
# once (`gpu_lost`, exit 3; `power_limit_lost`, exit 4), to be reported to
# the owner before anything else runs on the host.
#
# 1. whisper-cli on the clip with each model, on the CPU (-ng) and with
#    Vulkan: one `result` line each with the seconds, the speed against
#    the clip's length, the backend whisper loaded, the GPU's highest power
#    and load, and whether the clip's words came back.
# 2. RVP: `--selftest=speech` with Vulkan and the small model: the whole
#    file's subtitles (timed), then real time while it plays.
#
# -Vm leaves the GPU guard, the power limit and nvidia-smi out, for a try
# of this script on the Windows test VM (no GPU: Vulkan falls back to the
# CPU there); on the host the guard always runs.
param(
    [string]$Bundle = 'C:\Users\<user>\rvp-test\rvp-o',
    [string]$Data = 'C:\Users\<user>\rvp-test\m13-host\data',
    [string]$Out = 'C:\Users\<user>\rvp-test\m13-host\out',
    [string[]]$Models = @('base', 'small'),
    [int]$PowerLimit = 90,
    [switch]$Vm
)
$Models = $Models | ForEach-Object { $_ -split ',' } | Where-Object { $_ -ne '' }
$cli = "$Bundle\whisper\whisper-cli.exe"
$clip = "$Data\speech-long.wav"
New-Item -ItemType Directory -Force $Out | Out-Null
Write-Host "environment bundle=$Bundle data=$Data"
foreach ($needed in @($cli, $clip)) {
    if (-not (Test-Path $needed)) {
        Write-Host "Stopped: $needed is missing; nothing ran."
        exit 2
    }
}

# The card's power limit for the runs (see the header).
$script:limitWatts = 0
if ($PowerLimit -lt 100 -and -not $Vm) {
    $limitStatus = Join-Path $Out 'gpu-limit.txt'
    Remove-Item $limitStatus -ErrorAction SilentlyContinue
    & "$PSScriptRoot\gpu-limit.ps1" -Start -Percent $PowerLimit -Watch $PID -Status $limitStatus
    if ($LASTEXITCODE -ne 0) {
        Write-Host 'Stopped: the GPU power limit is not set; nothing ran.'
        exit 4
    }
    foreach ($line in Get-Content $limitStatus) {
        if ($line -match '^set .*limit=([\d.]+)') {
            $script:limitWatts = [math]::Max($script:limitWatts, [double]$Matches[1])
        }
    }
    Write-Host "environment power_limit_w=$script:limitWatts power_limit_percent=$PowerLimit"
}

# Stops everything when the GPU is gone or the limit went back up.
function Assert-Gpu([string]$when, [string]$text = '') {
    if ($Vm) { return }
    $smi = (& nvidia-smi --query-gpu=name --format=csv,noheader 2>&1 | Out-String).Trim()
    $lost = ($LASTEXITCODE -ne 0) -or ($smi -match 'lost|No devices|Unable to determine')
    $vulkan = $text -match 'GPU is lost|DEVICE_LOST|ErrorDeviceLost|VK_ERROR_INITIALIZATION_FAILED'
    if ($lost -or $vulkan) {
        Write-Host "gpu_lost $when nvidia-smi: $smi"
        Write-Host 'Stopped: report this to the owner before running anything else on the host.'
        exit 3
    }
    if ($script:limitWatts -gt 0) {
        $limits = & nvidia-smi --query-gpu=power.limit --format=csv,noheader,nounits 2>&1
        $above = @($limits | Where-Object { "$_".Trim() -notmatch '^[\d.]+$' -or [double]"$_".Trim() -gt $script:limitWatts + 0.5 })
        if ($above.Count -gt 0) {
            Write-Host "power_limit_lost $when nvidia-smi: $($limits -join ' ')"
            Write-Host 'Stopped: the GPU power limit is no longer set.'
            exit 4
        }
    }
}

# nvidia-smi logging the GPU's load and power to `$path` (none with -Vm).
function Start-GpuLog([string]$path) {
    if ($Vm) { return $null }
    Start-Process -FilePath 'nvidia-smi' -NoNewWindow -PassThru -RedirectStandardOutput $path `
        -ArgumentList '--query-gpu=utilization.gpu,power.draw', '--format=csv,noheader,nounits', '-lms', '250'
}

# The GPU's highest power (W) and load (%) in an nvidia-smi log.
function Read-GpuLog([string]$path) {
    $power = 0.0; $load = 0.0
    if (Test-Path $path) {
        foreach ($line in Get-Content $path) {
            $fields = "$line" -split ',' | ForEach-Object { $_.Trim() }
            if ($fields.Count -ge 2 -and $fields[0] -match '^[\d.]+$' -and $fields[1] -match '^[\d.]+$') {
                $load = [math]::Max($load, [double]$fields[0]); $power = [math]::Max($power, [double]$fields[1])
            }
        }
    }
    return @($power, $load)
}

# The clip's length in seconds (a PCM WAV file).
$bytes = [System.IO.File]::ReadAllBytes($clip)
$rate = [BitConverter]::ToInt32($bytes, 24); $perSecond = [BitConverter]::ToInt32($bytes, 28)
$clipSeconds = ($bytes.Length - 44) / $perSecond
Write-Host ("environment clip_seconds={0:N1} rate=$rate" -f $clipSeconds)
$gpuNow = if ($Vm) { '-' } else { (& nvidia-smi --query-gpu=utilization.gpu,power.draw --format=csv,noheader,nounits 2>&1 | Out-String).Trim() }
Write-Host "baseline gpu_util_power=$gpuNow"

foreach ($model in $Models) {
    $modelFile = "$Data\models\ggml-$model.bin"
    if (-not (Test-Path $modelFile)) {
        Write-Host "result whisper-$model skipped: no $modelFile"
        continue
    }
    foreach ($engine in @('cpu', 'vulkan')) {
        $name = "whisper-$model-$engine"
        Assert-Gpu "before $name"
        $base = "$Out\$name"
        Remove-Item "$base.srt", "$base.log", "$base.gpu.csv" -ErrorAction SilentlyContinue
        $arguments = @('-m', "`"$modelFile`"", '-f', "`"$clip`"", '-l', 'en', '-osrt', '-of', "`"$base`"")
        if ($engine -eq 'cpu') { $arguments += '-ng' }
        $smi = Start-GpuLog "$base.gpu.csv"
        $watch = [Diagnostics.Stopwatch]::StartNew()
        $process = Start-Process -FilePath $cli -ArgumentList $arguments -NoNewWindow -PassThru `
            -RedirectStandardError "$base.log" -RedirectStandardOutput "$base.out"
        $handle = $process.Handle
        if (-not $process.WaitForExit(600000)) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            Write-Host "timeout $name"
        }
        $watch.Stop()
        if ($smi) { Stop-Process -Id $smi.Id -ErrorAction SilentlyContinue }
        $log = Get-Content "$base.log" -Raw
        Assert-Gpu "after $name" $log
        $backend = (($log -split "`n") | Where-Object { $_ -match 'load_backend: loaded' } |
            ForEach-Object { ($_ -replace '.*loaded ', '' -replace ' backend.*', '').Trim() }) -join '+'
        $power, $load = Read-GpuLog "$base.gpu.csv"
        $text = if (Test-Path "$base.srt") { (Get-Content -Raw "$base.srt") } else { '' }
        $words = @('media player', 'subtitles', 'graphics card', 'ten') | Where-Object { $text -match "(?i)$_" }
        $seconds = $watch.Elapsed.TotalSeconds
        Write-Host ("result $name seconds={0:N1} speed={1:N1}x backend=$backend gpu_max_w={2:N0} gpu_max_load={3:N0} words={4}/4 exit={5}" -f `
            $seconds, ($clipSeconds / $seconds), $power, $load, $words.Count, $process.ExitCode)
    }
}

# RVP's window: the whole file and real time with Vulkan.
$model = if ($Models -contains 'small') { 'small' } else { $Models[0] }
$name = "rvp-speech-vulkan-$model"
Assert-Gpu "before $name"
$dir = Join-Path $Out $name
Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$dir\config", "$dir\state", "$dir\out" | Out-Null
Set-Content -Path "$dir\config\settings.json" -Value '{ "ipc-server": "" }' -Encoding ascii
$env:RVP_CONFIG_DIR = "$dir\config"
$env:RVP_STATE_DIR = "$dir\state"
$env:RVP_WHISPER_MODELS = "$Data\models"
if ($Vm) { $env:QT_OPENGL = 'software' }
$smi = Start-GpuLog "$dir\gpu.csv"
$arguments = @('--selftest=speech', '--speech-engine=vulkan', "--speech-model=$model", "--out=$dir\out",
    '--mpv-options=ao=null', "`"$clip`"")
$process = Start-Process -FilePath "$Bundle\rvp.exe" -ArgumentList $arguments -WorkingDirectory $Bundle -PassThru `
    -RedirectStandardOutput "$dir\rvp.log" -RedirectStandardError "$dir\rvp.err"
$handle = $process.Handle
if (-not $process.WaitForExit(600000)) {
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    Write-Host "timeout $name"
}
if ($smi) { Stop-Process -Id $smi.Id -ErrorAction SilentlyContinue }
Remove-Item Env:RVP_CONFIG_DIR, Env:RVP_STATE_DIR, Env:RVP_WHISPER_MODELS
$text = (Get-Content "$dir\rvp.log" -Raw) + (Get-Content "$dir\rvp.err" -Raw)
Assert-Gpu "after $name" $text
($text -split "`n") | Where-Object { $_ -match '^(check|metric speech|speech_|failures=|rvp: )' } |
    ForEach-Object { Write-Host $_.Trim() }
$power, $load = Read-GpuLog "$dir\gpu.csv"
Write-Host ("result $name exit={0} gpu_max_w={1:N0} gpu_max_load={2:N0}" -f $process.ExitCode, $power, $load)
