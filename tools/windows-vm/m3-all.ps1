# M3 checks on the Windows test VM (session 0 over SSH, software OpenGL):
# the functional self-test with a second instance (/add over the named
# pipe), then kill tests: RVP plays an album's second entry, is ended with
# Stop-Process -Force (TerminateProcess) and must reopen that entry paused
# within 5 s of the last position it reported.
#
# Layout: C:\rvp-test\rvp-bundle (rvp.exe, Qt, libmpv, fixture-*.mkv),
# C:\rvp-test\BlackSpace_en.dsf, this script and run.ps1 next to it.
$common = @('--out=C:\rvp-test\out', '--skin=C:\rvp-test\BlackSpace_en.dsf', '--mpv-options=ao=null')
$long = 'C:\rvp-test\rvp-bundle\fixture-60s.mkv'
$short = 'C:\rvp-test\rvp-bundle\fixture-3s.mkv'
function Run([string]$name, [string[]]$arguments) {
    Write-Host "== $name"
    $lines = & "$PSScriptRoot\run.ps1" -Extra $arguments | Select-String -NotMatch 'DPI|DpiAware|^grabbed=|^qt\.' | ForEach-Object { $_.Line }
    $lines | ForEach-Object { Write-Host $_ }
    return $lines
}
Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config, C:\rvp-test\out -ErrorAction SilentlyContinue
$null = Run 'functional (M3)' (@('--selftest', '--instance=vm-m3') + $common + @($long, $short))

$env:QT_OPENGL = 'software'
$env:RVP_CONFIG_DIR = 'C:\rvp-test\config'
$env:RVP_STATE_DIR = 'C:\rvp-test\state'
foreach ($run in 1..3) {
    Remove-Item -Recurse -Force C:\rvp-test\state -ErrorAction SilentlyContinue
    $log = "C:\rvp-test\kill-$run.log"
    $wait = 8 + 4 * $run
    $arguments = @('--selftest=playback', '--next', '--play-seconds=90') + $common + @($short, $long)
    $process = Start-Process -FilePath C:\rvp-test\rvp-bundle\rvp.exe -ArgumentList $arguments `
        -WorkingDirectory C:\rvp-test\rvp-bundle -RedirectStandardOutput $log `
        -RedirectStandardError "$log.err" -PassThru -NoNewWindow
    Start-Sleep -Seconds $wait
    Stop-Process -Id $process.Id -Force
    Start-Sleep -Seconds 1
    $last = Get-Content $log | Where-Object { $_ -like 'time=*' } | Select-Object -Last 1
    $time = [regex]::Match("$last", '^time=([0-9.]+)').Groups[1].Value
    if ($time -eq '') { $time = '0' }
    Write-Host "== kill run $run after $wait s, last $last"
    $null = Run "restart (expect $time s, entry 1 of 2)" (@('--selftest=resume', "--expect-time=$time",
        '--expect-tolerance=5', '--expect-entry=1', '--expect-entries=2') + $common)
}
