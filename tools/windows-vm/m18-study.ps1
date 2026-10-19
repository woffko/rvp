# M18's study (docs/spikes/m18-smoothing.md): crates/engine/examples/
# smoothing_bench.rs for each engine, preset and size, on the Windows VM or,
# with the owner's permission, on the host. The bench runs headless (no
# window, no sound).
#
# Copy the bench, built with
#   cargo xwin build --release --target x86_64-pc-windows-msvc \
#       -p rvp-engine --example smoothing_bench
# next to a libmpv-2.dll, the clips of tools/test-clips/make-clips.sh
# (smooth-square-1080p24.mkv, smooth-pan-{720,1080,2160}p24.mkv) into
# -Clips, and point -VsScript at RVP's own VapourSynth runtime (R73
# portable, embeddable Python 3.12, mvtools in its vs-plugins; never SVP's).
# -Rife names the folder of the RIFE plugin and its models; -Gpu samples
# nvidia-smi during each run (the host). The engine `fruc_vulkan` (FFmpeg's
# filter over NVIDIA's optical flow through Vulkan, not in RVP's libmpv)
# runs in the ffmpeg.exe of -Ffmpeg (a build of FFmpeg's master), its
# output measured the same way (the square through the bench's --raw).
#
# With -Gpu (the host) every run is guarded: nvidia-smi must see the GPU
# before and after it, and a lost GPU or a missing Vulkan device in a
# run's output stops the whole script at once with `gpu_lost` (exit 3),
# to be reported to the owner before anything else runs. On 2026-10-18
# RIFE at 2160p in RVP took the host's GPU down (a TDR it did not recover
# from, a reboot needed): RIFE at 2160p is left out unless -AllowRife4K,
# which needs the owner's separate permission.
#
# With -Gpu the runs also go at -PowerLimit percent (default 90) of the
# card's default power limit (D17): gpu-limit.ps1 asks for administrator
# rights once (a UAC window the owner answers), sets it, and puts the
# limit it found back when this script ends, a failed run included. No
# run starts until the limit is set, and a run stops the script when the
# limit has gone back up (exit 4). -PowerLimit 100 leaves the card alone.
#
# With -Videos ('label=PATH;label=PATH') and -At (seconds, one per video)
# the engines of -GrabEngines also write -Frames output frames of each
# video from its keyframe before that time (part C: the owner's clips,
# to look at the artefacts): the bench's grab mode, and for fruc_vulkan
# ffmpeg from the same keyframe. ffprobe (next to -Ffmpeg) gives each
# video's rate and height; RIFE skips 2160p videos unless -AllowRife4K.
# The frames show the owner's videos: they stay on the host and in
# ~/rvp-reference, never in the repository.
#
# -CounterTest names the shell's test binary (cargo xwin test --no-run
# -p rvp-shell, in a folder with RVP's DLLs): before the runs and 8 s
# into each speed run it prints Windows' adapter counter for every
# adapter (RVP_GPU_POWER_ADAPTERS=all) next to NVML's and nvidia-smi's
# figures (`counters` lines): the counter is AMD's only reading (D17).
#
# Prints one `result` line per run: the rate the chain sustains
# (`capacity_fps`, at mpv's top speed), its ratio to real time, the CPU
# load in cores, and for the square the edge error and ghost ratio.
param(
    [string]$Bench = 'C:\rvp-test\rvp-bundle\smoothing_bench.exe',
    [string]$Clips = 'C:\rvp-test\m18clips',
    [string]$VsScript = 'C:\rvp-spikes\s2-bundle\VSScript.dll',
    [string]$Rife = '',
    [string]$RifeModel = 'rife-v4.6_ensembleFalse',
    [string]$Ffmpeg = '',
    [string]$Out = 'C:\rvp-test\out\m18study',
    [string[]]$Engines = @('none', 'minterpolate', 'mvtools'),
    [string[]]$Presets = @('fast', 'medium', 'quality'),
    [string[]]$Sizes = @('720p', '1080p', '2160p'),
    [string]$Target = '2.5x',
    # VapourSynth's buffered and concurrent frames (0: the bench's 8).
    [int]$VsFrames = 0,
    [int]$Seconds = 20,
    [switch]$Gpu,
    [switch]$AllowRife4K,
    [int]$PowerLimit = 90,
    [string]$Videos = '',
    [string[]]$At = @(),
    [int]$Frames = 8,
    [string[]]$GrabEngines = @('none', 'mvtools', 'rife', 'fruc_vulkan'),
    [string[]]$GrabPresets = @('medium', 'quality'),
    [string]$CounterTest = ''
)
# `-File` hands a comma list over as one string.
$Engines = $Engines | ForEach-Object { $_ -split ',' } | Where-Object { $_ -ne '' }
$Presets = $Presets | ForEach-Object { $_ -split ',' } | Where-Object { $_ -ne '' }
$Sizes = $Sizes | ForEach-Object { $_ -split ',' } | Where-Object { $_ -ne '' }
$At = $At | ForEach-Object { $_ -split ',' } | Where-Object { $_ -ne '' }
$GrabEngines = $GrabEngines | ForEach-Object { $_ -split ',' } | Where-Object { $_ -ne '' }
$GrabPresets = $GrabPresets | ForEach-Object { $_ -split ',' } | Where-Object { $_ -ne '' }
$env:VSSCRIPT_PATH = $VsScript
New-Item -ItemType Directory -Force $Out | Out-Null
$cores = [Environment]::ProcessorCount
Write-Host "environment cores=$cores vsscript=$VsScript"

# The card's power limit for the runs (see the header).
$script:limitWatts = 0
if ($Gpu -and $PowerLimit -lt 100) {
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

# Stops everything when the GPU is gone (see the header).
function Assert-Gpu([string]$when, [string]$text = '') {
    if (-not $Gpu) { return }
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

# Windows' adapter counter next to NVML's and nvidia-smi's figures (see the
# header).
function Compare-Counters([string]$when) {
    if ($CounterTest -eq '') { return }
    $env:RVP_GPU_POWER_ADAPTERS = 'all'
    $lines = & $CounterTest gpu_power --nocapture 2>&1 | Where-Object { "$_" -match '^Sample' }
    Remove-Item Env:RVP_GPU_POWER_ADAPTERS
    $smi = (& nvidia-smi --query-gpu=power.draw,power.limit,utilization.gpu --format=csv,noheader 2>&1 | Out-String).Trim()
    Write-Host "counters $when nvidia-smi: $smi"
    foreach ($line in $lines) { Write-Host "counters $when $line" }
}

function Invoke-Bench([string]$name, [string[]]$arguments, [string]$program = $Bench, [string]$extra = '') {
    Assert-Gpu "before $name"
    $log = Join-Path $Out "$name.log"
    $gpuLog = Join-Path $Out "$name-gpu.csv"
    $smi = $null
    if ($Gpu) {
        $smi = Start-Process -FilePath 'nvidia-smi' -NoNewWindow -PassThru -RedirectStandardOutput $gpuLog `
            -ArgumentList '--query-gpu=utilization.gpu,memory.used,power.draw', '--format=csv,noheader,nounits', '-lms', '250'
    }
    $process = Start-Process -FilePath $program -ArgumentList $arguments -NoNewWindow -PassThru `
        -RedirectStandardOutput $log -RedirectStandardError "$log.err"
    $handle = $process.Handle  # keeps the times readable after the exit
    if ($CounterTest -ne '' -and $name -like 'speed-*' -and -not $process.WaitForExit(8000)) {
        Compare-Counters "during $name"
    }
    $process.WaitForExit()
    $cpu = $process.TotalProcessorTime.TotalSeconds
    $wall = ($process.ExitTime - $process.StartTime).TotalSeconds
    if ($smi) { Stop-Process -Id $smi.Id -ErrorAction SilentlyContinue }
    $text = (Get-Content $log -Raw) + $extra
    if ($program -ne $Bench) {
        # ffmpeg: the frames written and the time they took (-benchmark).
        $err = Get-Content "$log.err" -Raw
        $frames = [regex]::Matches($err, 'frame=\s*(\d+)') | Select-Object -Last 1
        $rtime = [regex]::Match($err, 'rtime=([\d.]+)s')
        if ($frames -and $rtime.Success -and [double]$rtime.Groups[1].Value -gt 0) {
            $capacity = [double]$frames.Groups[1].Value / [double]$rtime.Groups[1].Value
            $text += "`ncapacity_fps=$('{0:N1}' -f $capacity)`nrealtime_ratio=$('{0:N3}' -f ($capacity / $script:rate))"
        } elseif ($err -match '(?m)^.*(Error|error|failed).*$') {
            $text += "`nerror=$($Matches[0].Trim())"
        }
    }
    $fields = @()
    foreach ($key in 'realtime_ratio', 'capacity_fps', 'edge_error_mean', 'edge_error_max', 'ghost_ratio', 'error') {
        if ($text -match "(?m)^$key=(.*)$") { $fields += "$key=$($Matches[1].Trim())" }
    }
    $load = if ($wall -gt 0) { '{0:N2}' -f ($cpu / $wall) } else { '-' }
    $gpuText = ''
    if ($Gpu -and (Test-Path $gpuLog)) {
        # One array per sample, also when there is only one.
        $rows = @(Get-Content $gpuLog | ForEach-Object { , @($_ -split ',' | ForEach-Object { $_.Trim() }) })
        $samples = $rows | ForEach-Object { $_[0] } | Where-Object { $_ -match '^\d+$' }
        if ($samples) { $gpuText = ' gpu_mean={0:N0}' -f (($samples | Measure-Object -Average).Average) }
        $watts = $rows | Where-Object { $_.Count -ge 3 -and $_[2] -match '^[\d.]+$' } | ForEach-Object { [double]$_[2] }
        if ($watts) {
            $power = $watts | Measure-Object -Average -Maximum
            $gpuText += ' power_mean_w={0:N0} power_max_w={1:N0}' -f $power.Average, $power.Maximum
        }
    }
    Write-Host "result $name $($fields -join ' ') cpu_cores=$load wall_s=$('{0:N1}' -f $wall)$gpuText"
    $errText = if (Test-Path "$log.err") { Get-Content "$log.err" -Raw } else { '' }
    Assert-Gpu "after $name" ($text + $errText)
}

# The target's rate for 24000/1001 sources (ffmpeg's ratio).
$factor = if ($Target -match '^([\d.]+)x$') { [double]$Matches[1] } else { 0 }
$script:rate = if ($factor -gt 0) { 24000 * $factor / 1001 } else { [double]$Target }
$frucRate = if ($factor -gt 0) { "$([int](24000 * $factor))/1001" } else { $Target }
$perf = @{ fast = 'fast'; medium = 'medium'; quality = 'slow' }

Compare-Counters 'idle'

foreach ($engine in $Engines) {
    if ($engine -eq 'fruc_vulkan') {
        foreach ($preset in $Presets) {
            foreach ($size in $Sizes) {
                $arguments = @('-hide_banner', '-benchmark', '-nostdin', '-init_hw_device', 'vulkan=vk',
                    '-hwaccel', 'vulkan', '-hwaccel_output_format', 'vulkan', '-i', "$Clips\smooth-pan-${size}24.mkv",
                    '-an', '-vf', "fruc_vulkan=fps=${frucRate}:perf=$($perf[$preset])", '-f', 'null', '-')
                Invoke-Bench "speed-fruc_vulkan-$preset-$size" $arguments $Ffmpeg
            }
            $raw = "$Out\fruc_vulkan-$preset-square.raw"
            $arguments = @('-hide_banner', '-nostdin', '-y', '-init_hw_device', 'vulkan=vk', '-ss', '1',
                '-hwaccel', 'vulkan', '-hwaccel_output_format', 'vulkan', '-i', "$Clips\smooth-square-1080p24.mkv",
                '-an', '-vf', "fruc_vulkan=fps=48000/1001:perf=$($perf[$preset]),hwdownload,format=nv12,crop=1920:160:0:460,format=gray",
                '-frames:v', '12', '-f', 'rawvideo', $raw)
            Invoke-Bench "square-fruc_vulkan-$preset-render" $arguments $Ffmpeg
            Invoke-Bench "square-fruc_vulkan-$preset" @('--input', "$Clips\smooth-square-1080p24.mkv", '--mode', 'square',
                '--at', '1', '--frames', '12', '--band', '460:160', '--target', '2x', '--raw', $raw,
                '--raw-width', '1920', '--raw-start', '1.001', '--out', "$Out\fruc_vulkan-$preset-square")
        }
        continue
    }
    # PowerShell's names ignore case: not $presets.
    $runPresets = if ($engine -eq 'none') { @('-') } else { $Presets }
    foreach ($preset in $runPresets) {
        foreach ($size in $Sizes) {
            if ($engine -eq 'rife' -and $size -eq '2160p' -and $Gpu -and -not $AllowRife4K) {
                Write-Host "skipped speed-rife-$preset-$size (RIFE at 2160p needs -AllowRife4K)"
                continue
            }
            $arguments = @('--input', "$Clips\smooth-pan-${size}24.mkv", '--engine', $engine, '--mode', 'speed',
                '--seconds', $Seconds, '--target', $Target, '--out', "$Out\$engine-$preset-$size")
            if ($preset -ne '-') { $arguments += @('--preset', $preset) }
            if ($Rife -ne '') { $arguments += @('--rife', $Rife, '--rife-model', $RifeModel) }
            if ($VsFrames -gt 0) { $arguments += @('--vs-frames', $VsFrames) }
            $tag = if ($engine -eq 'rife') { "$engine-$preset-$RifeModel" } else { "$engine-$preset" }
            if ($VsFrames -gt 0) { $tag += "-vs$VsFrames" }
            Invoke-Bench "speed-$tag-$size" $arguments
        }
        if ($engine -ne 'none') {
            # 2x for the square: every other frame is made by the engine.
            $arguments = @('--input', "$Clips\smooth-square-1080p24.mkv", '--engine', $engine, '--mode', 'square',
                '--at', '1', '--frames', '12', '--band', '460:160', '--target', '2x',
                '--out', "$Out\$engine-$preset-square")
            if ($preset -ne '-') { $arguments += @('--preset', $preset) }
            if ($Rife -ne '') { $arguments += @('--rife', $Rife, '--rife-model', $RifeModel) }
            $tag = if ($engine -eq 'rife') { "$engine-$preset-$RifeModel" } else { "$engine-$preset" }
            Invoke-Bench "square-$tag" $arguments
        }
    }
}

# Part C: frames of other videos through each engine (see the header).
$videoList = @($Videos -split ';' | Where-Object { $_ -ne '' })
if ($videoList.Count -gt 0 -and ($At.Count -ne $videoList.Count -or $Ffmpeg -eq '')) {
    Write-Host 'Stopped: -Videos needs one -At time per video and -Ffmpeg (for ffprobe).'
    exit 2
}
$ffprobe = if ($Ffmpeg -ne '') { Join-Path (Split-Path $Ffmpeg) 'ffprobe.exe' } else { '' }
for ($index = 0; $index -lt $videoList.Count; $index++) {
    $label, $video = $videoList[$index] -split '=', 2
    $time = $At[$index]
    $probe = & $ffprobe -v error -select_streams v:0 -show_entries stream=r_frame_rate,height -of csv=p=0 $video
    $fields = "$probe".Trim() -split ','
    $height = [int]($fields | Where-Object { $_ -match '^\d+$' } | Select-Object -First 1)
    $rate = $fields | Where-Object { $_ -match '^\d+/\d+$' } | Select-Object -First 1
    if (-not $rate) {
        Write-Host "skipped ${label}: ffprobe found no video rate ($probe)"
        continue
    }
    $num, $den = $rate -split '/'
    Write-Host "video $label height=$height rate=$rate at=$time"
    foreach ($engine in $GrabEngines) {
        $runPresets = if ($engine -eq 'none' -or $engine -eq 'rife') { @('-') } else { $GrabPresets }
        if ($engine -eq 'rife' -and $height -ge 2000 -and -not $AllowRife4K) {
            Write-Host "skipped grab-$label-rife (RIFE at 2160p needs -AllowRife4K)"
            continue
        }
        foreach ($preset in $runPresets) {
            $name = if ($preset -eq '-') { "grab-$label-$engine" } else { "grab-$label-$engine-$preset" }
            $dir = Join-Path $Out $name
            New-Item -ItemType Directory -Force $dir | Out-Null
            if ($engine -eq 'fruc_vulkan') {
                $arguments = @('-hide_banner', '-nostdin', '-y', '-init_hw_device', 'vulkan=vk', '-noaccurate_seek',
                    '-ss', $time, '-hwaccel', 'vulkan', '-hwaccel_output_format', 'vulkan', '-i', $video, '-an',
                    '-vf', "fruc_vulkan=fps=$([int64]$num * 2)/${den}:perf=$($perf[$preset]),hwdownload,format=nv12",
                    '-frames:v', $Frames, "$dir\grab-%03d.png")
                Invoke-Bench $name $arguments $Ffmpeg
                continue
            }
            $arguments = @('--input', $video, '--engine', $engine, '--mode', 'grab', '--at', $time,
                '--frames', $Frames, '--target', '2x', '--source-fps', $rate, '--out', $dir)
            if ($preset -ne '-') { $arguments += @('--preset', $preset) }
            if ($engine -eq 'rife' -and $Rife -ne '') { $arguments += @('--rife', $Rife, '--rife-model', $RifeModel) }
            Invoke-Bench $name $arguments
        }
    }
}
