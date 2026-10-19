# M18's host session (goal T): RVP's motion smoothing in RVP's own window
# on the owner's desktop and RTX card, run only with the owner's permission
# after advance notice, every time. RVP's window appears for each run
# (about -Seconds plus 15 s, mpv's sound off) and a UAC window asks once
# for the power limit.
#
# -Bundle is a portable RVP (tools/packaging/package.py's folder, with its
# own VapourSynth runtime in vapoursynth\); -Clips holds the clips of
# tools/test-clips/make-clips.sh (smooth-pan-720p24.mkv and
# smooth-pan-1080p24.mkv). gpu-limit.ps1 must be next to this script.
#
# Every run goes at -PowerLimit percent (default 90) of the card's default
# power limit through gpu-limit.ps1 (D17), which puts the limit it found
# back when this script ends, and is guarded like m18-study.ps1's GPU runs:
# nvidia-smi must see the card before and after it, and a lost GPU, a
# missing Vulkan device or a limit that went back up stops the script at
# once (`gpu_lost`, exit 3; `power_limit_lost`, exit 4), to be reported to
# the owner before anything else runs on the host. No RIFE at 2160p.
#
# Each run is `rvp --selftest=playback` with settings of its own (the
# renderer, smoothing on with an engine and preset, at the display's rate)
# and prints one `result` line: the plan RVP chose, the rate mpv's filters
# reached against the file's, the frames dropped (all, and after the first
# 10 s), the GPU's mean load and power, RVP's exit code and the crash logs
# RVP wrote (RIFE's plugin crashes at exit unless RVP ends its process
# first); a `notices` line lists RVP's notices (`rvp: ` lines: the steps
# of RVP's GPU power budget, on at its default 90 %, or an output that
# could not start). The run `svp-1080p` keeps SVP's pipe (mpvpipe) and
# needs SVP 4 running on the desktop: SVP attaches and RVP's smoothing
# stays off.
#
# -Vm leaves the GPU guard, the power limit and nvidia-smi out and takes
# software OpenGL, for a try of this script on the Windows test VM (no GPU
# and no swapchain in its session 0: `-Runs mvtools-opengl-1080p`); on the
# host the guard always runs.
param(
    [string]$Bundle = 'C:\Users\<user>\rvp-test\rvp-t',
    [string]$Clips = 'C:\Users\<user>\rvp-test\m18-study\clips',
    [string]$Out = 'C:\Users\<user>\rvp-test\m18-host',
    [string[]]$Runs = @(),
    [int]$Seconds = 30,
    [int]$PowerLimit = 90,
    [switch]$Vm
)
# `-File` hands a comma list over as one string.
$Runs = $Runs | ForEach-Object { $_ -split ',' } | Where-Object { $_ -ne '' }

# name, renderer, engine, preset, clip
$table = @(
    @('blend-d3d11-1080p', 'direct3d11', 'blend', 'medium', 'smooth-pan-1080p24.mkv'),
    @('mvtools-opengl-1080p', 'opengl', 'mvtools', 'medium', 'smooth-pan-1080p24.mkv'),
    @('mvtools-d3d11-1080p', 'direct3d11', 'mvtools', 'medium', 'smooth-pan-1080p24.mkv'),
    @('rife-d3d11-720p', 'direct3d11', 'rife', 'medium', 'smooth-pan-720p24.mkv'),
    @('rife-d3d11-1080p', 'direct3d11', 'rife', 'medium', 'smooth-pan-1080p24.mkv'),
    @('rife-quality-d3d11-1080p', 'direct3d11', 'rife', 'quality', 'smooth-pan-1080p24.mkv'),
    @('fruc-vulkan-720p', 'vulkan', 'fruc-vulkan', 'medium', 'smooth-pan-720p24.mkv'),
    @('fruc-vulkan-1080p', 'vulkan', 'fruc-vulkan', 'medium', 'smooth-pan-1080p24.mkv'),
    @('svp-1080p', 'direct3d11', 'mvtools', 'medium', 'smooth-pan-1080p24.mkv')
)
if ($Runs.Count -gt 0) {
    $unknown = @($Runs | Where-Object { $name = $_; -not ($table | Where-Object { $_[0] -eq $name }) })
    if ($unknown.Count -gt 0) {
        Write-Host "Stopped: unknown runs $($unknown -join ', ')"
        exit 2
    }
    $table = @($table | Where-Object { $Runs -contains $_[0] })
}
New-Item -ItemType Directory -Force $Out | Out-Null
if ($Vm) { $env:QT_OPENGL = 'software' }
Write-Host "environment bundle=$Bundle clips=$Clips"

# The card's power limit for the runs (see the header).
$script:limitWatts = 0
if ($PowerLimit -lt 100 -and -not $Vm) {
    $limitStatus = Join-Path $Out 'gpu-limit.txt'
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

# Stops everything when the GPU is gone or the limit went back up (see the
# header).
function Assert-Gpu([string]$when, [string]$text = '') {
    if ($Vm) { return }
    $smi = (& nvidia-smi --query-gpu=name --format=csv,noheader 2>&1 | Out-String).Trim()
    $lost = ($LASTEXITCODE -ne 0) -or ($smi -match 'lost|No devices|Unable to determine')
    $vulkan = $text -match 'GPU is lost|DEVICE_LOST|No devices found|VK_ERROR_INITIALIZATION_FAILED'
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

foreach ($run in $table) {
    $name, $renderer, $engine, $preset, $clip = $run
    Assert-Gpu "before $name"
    $dir = Join-Path $Out $name
    Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force "$dir\config", "$dir\state", "$dir\out" | Out-Null
    $settings = [ordered]@{
        render = @{ renderer = $renderer }
        smoothing = [ordered]@{ enabled = $true; engine = $engine; preset = $preset; target = 'display' }
    }
    # Only the SVP run keeps SVP's pipe.
    if ($name -notlike 'svp-*') { $settings['ipc-server'] = '' }
    $settings | ConvertTo-Json -Depth 4 | Set-Content -Path "$dir\config\settings.json" -Encoding ascii
    $arguments = @('--selftest=playback', "--play-seconds=$Seconds", "--out=$dir\out", '--mpv-options=ao=null')
    if ($name -like 'svp-*') { $arguments += '--svp' }
    $arguments += "`"$Clips\$clip`""

    $env:RVP_CONFIG_DIR = "$dir\config"
    $env:RVP_STATE_DIR = "$dir\state"
    $gpuLog = "$dir\gpu.csv"
    $smi = $null
    if (-not $Vm) {
        $smi = Start-Process -FilePath 'nvidia-smi' -NoNewWindow -PassThru -RedirectStandardOutput $gpuLog `
            -ArgumentList '--query-gpu=utilization.gpu,memory.used,power.draw', '--format=csv,noheader,nounits', '-lms', '250'
    }
    $process = Start-Process -FilePath "$Bundle\rvp.exe" -ArgumentList $arguments -WorkingDirectory $Bundle -PassThru `
        -RedirectStandardOutput "$dir\rvp.log" -RedirectStandardError "$dir\rvp.err"
    $handle = $process.Handle  # keeps the exit code readable
    if (-not $process.WaitForExit(($Seconds + 90) * 1000)) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        Write-Host "timeout $name"
    }
    $process.WaitForExit()
    if ($smi) { Stop-Process -Id $smi.Id -ErrorAction SilentlyContinue }
    Remove-Item Env:RVP_CONFIG_DIR, Env:RVP_STATE_DIR

    $text = (Get-Content "$dir\rvp.log" -Raw) + (Get-Content "$dir\rvp.err" -Raw)
    $fields = @("exit=$($process.ExitCode)")
    if ($text -match '(?m)^smoothing=(.*)$') { $fields += "plan=`"$($Matches[1].Trim())`"" }
    if ($text -match '(?m)^diagnostics_end=(.*)$') {
        $d = $Matches[1] | ConvertFrom-Json
        $fields += 'source_fps={0:N3} filter_fps={1:N3}' -f $d.container_fps, $d.estimated_vf_fps
        $fields += "vf=`"$($d.vf)`""
    }
    if ($text -match '(?m)^frames_dropped=(\d+) decoder_dropped=(\d+)') {
        $fields += "dropped=$($Matches[1]) decoder_dropped=$($Matches[2])"
    }
    if ($text -match '(?m)^frame_drop_timeline=([\d,]+)') {
        $timeline = @($Matches[1] -split ',' | ForEach-Object { [int]$_ })
        if ($timeline.Count -gt 10) { $fields += "dropped_after_10s=$($timeline[-1] - $timeline[9])" }
    }
    $failed = @([regex]::Matches($text, '(?m)^check (\S+)=FAIL') | ForEach-Object { $_.Groups[1].Value })
    $fields += "failed_checks=$(if ($failed) { $failed -join ',' } else { 'none' })"
    $crashes = @(Get-ChildItem "$dir\state" -Filter 'crash-*' -ErrorAction SilentlyContinue | ForEach-Object { $_.Name })
    $fields += "crash_logs=$(if ($crashes) { $crashes -join ',' } else { 'none' })"
    $rows = @(Get-Content $gpuLog -ErrorAction SilentlyContinue | ForEach-Object { , @($_ -split ',' | ForEach-Object { $_.Trim() }) })
    $load = $rows | ForEach-Object { $_[0] } | Where-Object { $_ -match '^\d+$' }
    if ($load) { $fields += 'gpu_mean={0:N0}' -f (($load | Measure-Object -Average).Average) }
    $watts = $rows | Where-Object { $_.Count -ge 3 -and $_[2] -match '^[\d.]+$' } | ForEach-Object { [double]$_[2] }
    if ($watts) {
        $power = $watts | Measure-Object -Average -Maximum
        $fields += 'power_mean_w={0:N0} power_max_w={1:N0}' -f $power.Average, $power.Maximum
    }
    Write-Host "result $name $($fields -join ' ')"
    # RVP's notices (`rvp: ` on standard error).
    $notices = @([regex]::Matches($text, '(?m)^rvp: (.*)$') | ForEach-Object { $_.Groups[1].Value.Trim() })
    if ($notices) { Write-Host "notices $name $($notices -join ' | ')" }
    Assert-Gpu "after $name" $text
}
