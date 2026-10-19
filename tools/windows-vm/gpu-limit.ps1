# Host runs at a lower GPU power limit (owner decision 2026-10-19, D17):
# the one system setting RVP's test scripts change on the Windows host, and
# only for the length of a run the owner allowed. On 2026-10-18 RIFE at
# 2160p took the host's GPU down; GPU runs there now go at 90 % of the
# card's default power limit.
#
# It asks for administrator rights once (a UAC window on the owner's
# desktop: the owner clicks Yes), sets each NVIDIA card's limit to
# -Percent of its default with nvidia-smi, and then waits, hidden, until
# the process it watches ends, however it ends (a failed or cancelled run
# included), to put back the limit it found. A reboot also puts the
# default back.
#
#   gpu-limit.ps1 -Percent 90 -Run C:\Users\<user>\rvp-test\run-t-rife.cmd
#       limits the cards, runs the command, and leaves when it ends
#   gpu-limit.ps1 -Start -Percent 90 -Watch PID -Status FILE
#       limits the cards until process PID ends (m18-study.ps1 -Gpu);
#       exits 0 once the limit is set, 4 when it is not
#
# The elevated copy (-Elevated) writes `set`, `error` and `restored` lines
# to the status file; nothing else on the host changes.
param(
    [int]$Percent = 90,
    [string]$Run = '',
    [string[]]$RunArguments = @(),
    [switch]$Start,
    [int]$Watch = 0,
    [string]$Status = '',
    [switch]$Elevated
)

function Read-Cards {
    $text = & nvidia-smi --query-gpu=index,power.limit,power.default_limit --format=csv,noheader,nounits 2>&1
    if ($LASTEXITCODE -ne 0) { throw "nvidia-smi: $text" }
    foreach ($line in $text) {
        $fields = "$line" -split ',' | ForEach-Object { $_.Trim() }
        if ($fields.Count -ge 3 -and $fields[0] -match '^\d+$') {
            [pscustomobject]@{ Index = [int]$fields[0]; Limit = [double]$fields[1]; Default = [double]$fields[2] }
        }
    }
}

function Write-Status([string]$line) {
    Add-Content -Path $Status -Value $line
}

if ($Elevated) {
    # The elevated copy: set, wait for the watched process, put back.
    $found = @()
    try {
        $found = @(Read-Cards)
        if ($found.Count -eq 0) { throw 'no NVIDIA card' }
        foreach ($card in $found) {
            $target = [math]::Floor($card.Default * $Percent / 100)
            if ($card.Limit -gt $target) {
                $out = & nvidia-smi -i $card.Index -pl $target 2>&1
                if ($LASTEXITCODE -ne 0) { throw "nvidia-smi -pl ${target}: $out" }
            }
        }
        foreach ($card in @(Read-Cards)) {
            $target = [math]::Floor($card.Default * $Percent / 100)
            if ($card.Limit -gt $target + 0.5) { throw "card $($card.Index) still at $($card.Limit) W" }
            Write-Status "set card=$($card.Index) limit=$($card.Limit) default=$($card.Default)"
        }
    } catch {
        Write-Status "error $_"
    }
    if ($Watch -gt 0) {
        Wait-Process -Id $Watch -ErrorAction SilentlyContinue
    }
    foreach ($card in $found) {
        $out = & nvidia-smi -i $card.Index -pl $card.Limit 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Status "restored card=$($card.Index) limit=$($card.Limit)"
        } else {
            Write-Status "error restoring card $($card.Index): $out"
        }
    }
    exit 0
}

if ($Run -ne '') {
    $Watch = $PID
    if ($Status -eq '') { $Status = Join-Path $env:TEMP "rvp-gpu-limit-$PID.txt" }
    $Start = $true
}
if (-not $Start) {
    Write-Host 'usage: gpu-limit.ps1 -Percent 90 -Run COMMAND | -Start -Percent 90 -Watch PID -Status FILE'
    exit 2
}
if ($Watch -le 0 -or $Status -eq '') {
    Write-Host 'gpu-limit: -Start needs -Watch and -Status'
    exit 2
}
Remove-Item -Path $Status -ErrorAction SilentlyContinue
$arguments = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$PSCommandPath`" -Elevated " +
    "-Percent $Percent -Watch $Watch -Status `"$Status`""
try {
    # The UAC window: the owner says yes on the desktop.
    Start-Process -FilePath 'powershell.exe' -Verb RunAs -WindowStyle Hidden -ArgumentList $arguments | Out-Null
} catch {
    Write-Host "gpu_limit not_set: no administrator rights ($_)"
    exit 4
}
$deadline = (Get-Date).AddSeconds(120)
$lines = @()
while ((Get-Date) -lt $deadline) {
    if (Test-Path $Status) {
        $lines = @(Get-Content $Status)
        if ($lines | Where-Object { $_ -match '^(set|error)' }) { break }
    }
    Start-Sleep -Milliseconds 500
}
$errors = @($lines | Where-Object { $_ -match '^error' })
$set = @($lines | Where-Object { $_ -match '^set' })
if ($errors.Count -gt 0 -or $set.Count -eq 0) {
    Write-Host "gpu_limit not_set: $(if ($errors) { $errors -join '; ' } else { 'no answer in 120 s' })"
    exit 4
}
foreach ($line in $set) { Write-Host "gpu_limit $line" }

if ($Run -ne '') {
    $command = @{ FilePath = $Run; NoNewWindow = $true; PassThru = $true; Wait = $true }
    # Start-Process refuses an empty argument list.
    if ($RunArguments.Count -gt 0) { $command.ArgumentList = $RunArguments }
    $process = Start-Process @command
    Write-Host "gpu_limit run exit=$($process.ExitCode); the limit comes back when this script ends"
    exit $process.ExitCode
}
exit 0
