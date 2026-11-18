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
# could not start). The runs `svp-*` keep SVP's pipe (mpvpipe) and need
# SVP 4 running on the desktop: SVP attaches and RVP's smoothing stays off
# (`svp-off-1080p` has it turned off). With -CompareBundle (a copy of the
# bundle with another libmpv-2.dll, such as shinchiro's),
# `svp-1080p-compare` repeats `svp-1080p` there.
#
# The CPU counts as much as the GPU: MVTools runs on it. Before the runs a
# `baseline` line gives the machine's CPU load over 5 s, the busiest
# processes (in cores) and the GPU's load and power; each `result` line
# adds the CPU's mean load during the run and RVP's own share (in cores).
# mpv's log of each run is mpv.log in its folder.
#
# -Vm leaves the GPU guard, the power limit and nvidia-smi out and takes
# software OpenGL, for a try of this script on the Windows test VM (no GPU
# and no swapchain in its session 0: `-Runs mvtools-opengl-1080p`); on the
# host the guard always runs.
param(
    [string]$Bundle = "$env:USERPROFILE\rvp-test\rvp-t",
    [string]$Clips = "$env:USERPROFILE\rvp-test\m18-study\clips",
    [string]$Out = "$env:USERPROFILE\rvp-test\m18-host",
    [string[]]$Runs = @(),
    # The clips are 20 s long: a run plays less than that.
    [int]$Seconds = 15,
    [int]$PowerLimit = 90,
    [string]$CompareBundle = '',
    # Loops the clips (mpv's loop-file) for runs longer than they are: the
    # dropped frames' steps take about 12 s each.
    [switch]$Loop,
    [switch]$Vm
)
# `-File` hands a comma list over as one string.
$Runs = $Runs | ForEach-Object { $_ -split ',' } | Where-Object { $_ -ne '' }

# name, renderer, engine, preset, clip[, the bundle: main or compare]
$table = @(
    @('blend-d3d11-1080p', 'direct3d11', 'blend', 'medium', 'smooth-pan-1080p24.mkv'),
    @('mvtools-opengl-1080p', 'opengl', 'mvtools', 'medium', 'smooth-pan-1080p24.mkv'),
    @('mvtools-d3d11-1080p', 'direct3d11', 'mvtools', 'medium', 'smooth-pan-1080p24.mkv'),
    @('rife-d3d11-720p', 'direct3d11', 'rife', 'medium', 'smooth-pan-720p24.mkv'),
    @('rife-d3d11-1080p', 'direct3d11', 'rife', 'medium', 'smooth-pan-1080p24.mkv'),
    @('rife-quality-d3d11-1080p', 'direct3d11', 'rife', 'quality', 'smooth-pan-1080p24.mkv'),
    @('fruc-vulkan-720p', 'vulkan', 'fruc-vulkan', 'medium', 'smooth-pan-720p24.mkv'),
    @('fruc-vulkan-1080p', 'vulkan', 'fruc-vulkan', 'medium', 'smooth-pan-1080p24.mkv'),
    @('svp-1080p', 'direct3d11', 'mvtools', 'medium', 'smooth-pan-1080p24.mkv'),
    @('svp-off-1080p', 'direct3d11', 'off', 'medium', 'smooth-pan-1080p24.mkv'),
    @('svp-1080p-compare', 'direct3d11', 'mvtools', 'medium', 'smooth-pan-1080p24.mkv', 'compare')
)
if ($CompareBundle -eq '') { $table = @($table | Where-Object { $_.Count -lt 6 }) }
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

# The CPU's load in percent of all its threads (WMI's counters, whose names
# Windows does not translate).
function Get-CpuPercent {
    [double](Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'").PercentProcessorTime
}

# The background before the runs (see the header).
$cores = [Environment]::ProcessorCount
$before = @{}
Get-Process | ForEach-Object { if ($_.TotalProcessorTime) { $before["$($_.Id)"] = $_.TotalProcessorTime.TotalSeconds } }
$started = Get-Date
$samples = @(1..5 | ForEach-Object { Start-Sleep -Seconds 1; Get-CpuPercent })
$elapsed = ((Get-Date) - $started).TotalSeconds
$busy = Get-Process | ForEach-Object {
    $start = $before["$($_.Id)"]
    if ($null -ne $start -and $_.TotalProcessorTime) {
        [pscustomobject]@{ Name = $_.ProcessName; Cores = ($_.TotalProcessorTime.TotalSeconds - $start) / $elapsed }
    }
} | Group-Object Name | ForEach-Object {
    [pscustomobject]@{ Name = $_.Name; Cores = ($_.Group | Measure-Object Cores -Sum).Sum }
} | Sort-Object Cores -Descending | Select-Object -First 8
$top = ($busy | Where-Object { $_.Cores -ge 0.05 } | ForEach-Object { '{0}:{1:N2}' -f $_.Name, $_.Cores }) -join ','
$gpuNow = if ($Vm) { '-' } else { (& nvidia-smi --query-gpu=utilization.gpu,power.draw --format=csv,noheader,nounits 2>&1 | Out-String).Trim() }
Write-Host ("baseline threads=$cores cpu_percent={0:N1} top=$top gpu_util_power=$gpuNow" -f ($samples | Measure-Object -Average).Average)

foreach ($run in $table) {
    $name, $renderer, $engine, $preset, $clip, $which = $run
    $bundleDir = if ($which -eq 'compare') { $CompareBundle } else { $Bundle }
    Assert-Gpu "before $name"
    $dir = Join-Path $Out $name
    Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force "$dir\config", "$dir\state", "$dir\out" | Out-Null
    $settings = [ordered]@{
        render = @{ renderer = $renderer }
        smoothing = [ordered]@{ enabled = ($engine -ne 'off'); engine = $(if ($engine -eq 'off') { 'mvtools' } else { $engine })
            preset = $preset; target = 'display' }
    }
    # Only the SVP run turns SVP support on (off by default) on its pipe.
    if ($name -like 'svp-*') { $settings['svp-enabled'] = $true } else { $settings['ipc-server'] = '' }
    $settings | ConvertTo-Json -Depth 4 | Set-Content -Path "$dir\config\settings.json" -Encoding ascii
    $mpvOptions = "ao=null,log-file=$dir\mpv.log"
    if ($Loop) { $mpvOptions += ',loop-file=inf' }
    $arguments = @('--selftest=playback', "--play-seconds=$Seconds", "--out=$dir\out", "--mpv-options=$mpvOptions")
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
    $process = Start-Process -FilePath "$bundleDir\rvp.exe" -ArgumentList $arguments -WorkingDirectory $bundleDir -PassThru `
        -RedirectStandardOutput "$dir\rvp.log" -RedirectStandardError "$dir\rvp.err"
    $handle = $process.Handle  # keeps the exit code and times readable
    $deadline = (Get-Date).AddSeconds($Seconds + 90)
    $cpu = @()
    while (-not $process.WaitForExit(2000)) {
        $cpu += Get-CpuPercent
        if ((Get-Date) -gt $deadline) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            Write-Host "timeout $name"
            break
        }
    }
    $process.WaitForExit()
    $wall = ($process.ExitTime - $process.StartTime).TotalSeconds
    if ($smi) { Stop-Process -Id $smi.Id -ErrorAction SilentlyContinue }
    Remove-Item Env:RVP_CONFIG_DIR, Env:RVP_STATE_DIR

    $text = (Get-Content "$dir\rvp.log" -Raw) + (Get-Content "$dir\rvp.err" -Raw)
    $fields = @("exit=$($process.ExitCode)")
    if ($bundleDir -ne $Bundle) { $fields += "bundle=$bundleDir" }
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
    # When RVP's smoothing filter went in (each dropped frames' step puts
    # it in again), in seconds of mpv's log.
    $steps = @(Select-String -Path "$dir\mpv.log" -Pattern '^\[\s*([\d.]+)\].*operation="pre", value="@rvpsmooth' -ErrorAction SilentlyContinue |
        ForEach-Object { '{0:N1}' -f [double]$_.Matches[0].Groups[1].Value })
    if ($steps) { $fields += "filter_at=$($steps -join ',')" }
    if ($cpu) { $fields += 'cpu_percent={0:N0}' -f ($cpu | Measure-Object -Average).Average }
    if ($wall -gt 0) { $fields += 'rvp_cores={0:N2}' -f ($process.TotalProcessorTime.TotalSeconds / $wall) }
    Write-Host "result $name $($fields -join ' ')"
    # RVP's notices (`rvp: ` on standard error).
    $notices = @([regex]::Matches($text, '(?m)^rvp: (.*)$') | ForEach-Object { $_.Groups[1].Value.Trim() })
    if ($notices) { Write-Host "notices $name $($notices -join ' | ')" }
    Assert-Gpu "after $name" $text
}
