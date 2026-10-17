# M11 checks on the Windows test VM (session 0 over SSH, software OpenGL,
# no sound device). Copy this file and run.ps1 to C:\rvp-test with the
# bundle in C:\rvp-test\rvp-bundle and M11's fixtures next to rvp.exe
# (testdata/media: fixture-60s.mkv, fixture-3s.mkv, fixture-marker.mkv,
# fixture-chapters.mkv and .srt, and the dvd folder), and the headless
# tests built for Windows next to the bundle's libmpv-2.dll:
#   cargo xwin test --no-run --release --target x86_64-pc-windows-msvc \
#       -p rvp-engine --test thumbnails
#   cargo xwin test --no-run --release --target x86_64-pc-windows-msvc \
#       -p rvp-app --test m11
# copied as rvp-bundle\thumbnails.exe and rvp-bundle\m11.exe.
#
# 1. The M11 self-test (`--selftest=m11`, shell/qml/SelfTestM11.qml), with
#    the command line's /seek and /title switches.
# 2. The power actions only through the dry-run hook: the self-test's
#    "shut down" is written to the state folder's power-dry-run.txt, and
#    the VM's boot time is the same before and after.
# 3. The Recycle Bin: the self-test's own file (recycle-me.mkv in the out
#    folder) is in this user's Recycle Bin after the run; it is removed
#    from there afterwards.
# 4. The thumbnail core's and M11's headless tests on Windows' FFmpeg,
#    with RVP_TESTDATA pointing at the bundle's fixtures.
param([string]$Exe = 'C:\rvp-test\rvp-bundle\rvp.exe')
$bundle = Split-Path $Exe
$filter = 'DPI|DpiAware|^grabbed=|^qt\.'
$out = 'C:\rvp-test\out\m11'
$recycled = "$out\recycle-me.mkv"

# The entries of this user's Recycle Bin that came from `$Path`: the
# `$I` files, which hold the original path.
function Get-RecycledFrom([string]$Path) {
    $sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $bin = "C:\`$Recycle.Bin\$sid"
    $found = @()
    if (-not (Test-Path -LiteralPath $bin)) { return $found }
    foreach ($info in Get-ChildItem -LiteralPath $bin -Force -Filter '$I*') {
        $bytes = [System.IO.File]::ReadAllBytes($info.FullName)
        if ($bytes.Length -lt 28) { continue }
        # Version 1 (Vista to 8.1): a 520-byte path at 24; version 2: its
        # length at 24, the path at 28.
        $version = [BitConverter]::ToInt64($bytes, 0)
        $at = if ($version -eq 1) { 24 } else { 28 }
        $original = [System.Text.Encoding]::Unicode.GetString($bytes, $at, $bytes.Length - $at).TrimEnd([char]0)
        if ($original -ieq $Path) { $found += $info }
    }
    return $found
}

$boot = (Get-CimInstance Win32_OperatingSystem).LastBootUpTime
Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config, $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force C:\rvp-test\config, $out | Out-Null
Set-Content -Path C:\rvp-test\config\settings.json -Value '{ "ipc-server": "" }' -Encoding ascii
foreach ($old in (Get-RecycledFrom $recycled)) {
    Remove-Item -LiteralPath $old.FullName, (Join-Path $old.DirectoryName ('$R' + $old.Name.Substring(2))) `
        -Force -ErrorAction SilentlyContinue
}

Write-Host '== M11 self-test'
$extra = @('--selftest=m11', "--out=$out", "--dvd=$bundle\dvd", "--audio=$bundle\fixture-3s.mkv",
    "--clip=$bundle\fixture-60s.mkv", '--mpv-options=ao=null', '/seek=0:00:03', '/title=M11 title',
    "$bundle\fixture-chapters.mkv", "$bundle\fixture-60s.mkv", "$bundle\fixture-3s.mkv")
& "$PSScriptRoot\run.ps1" -Exe $Exe -Extra $extra | Select-String -NotMatch $filter |
    ForEach-Object { Write-Host $_.Line }

Write-Host '== power actions (dry run)'
$dry = 'C:\rvp-test\state\power-dry-run.txt'
$text = if (Test-Path $dry) { (Get-Content -Raw $dry).Trim() } else { '' }
Write-Host "power_dry_run=$text"
Write-Host "check power_dry_run_file=$(if ($text -match 'shut down') { 'PASS' } else { 'FAIL' })"
$sameBoot = (Get-CimInstance Win32_OperatingSystem).LastBootUpTime -eq $boot
Write-Host "check vm_kept_running=$(if ($sameBoot) { 'PASS' } else { 'FAIL' })"

Write-Host '== Recycle Bin'
$entries = @(Get-RecycledFrom $recycled)
Write-Host "check recycled_file_gone=$(if (-not (Test-Path -LiteralPath $recycled)) { 'PASS' } else { 'FAIL' })"
Write-Host "check recycled_file_in_bin=$(if ($entries.Count -ge 1) { 'PASS' } else { 'FAIL' })"
foreach ($entry in $entries) {
    Remove-Item -LiteralPath $entry.FullName, (Join-Path $entry.DirectoryName ('$R' + $entry.Name.Substring(2))) `
        -Force -ErrorAction SilentlyContinue
}

$env:RVP_TESTDATA = $bundle
foreach ($test in @('thumbnails', 'm11')) {
    Write-Host "== headless tests: $test"
    $binary = "$bundle\$test.exe"
    if (-not (Test-Path $binary)) {
        Write-Host "check headless_$test=SKIPPED no $test.exe"
        continue
    }
    Push-Location $bundle
    $lines = & $binary --test-threads=1 --nocapture 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Pop-Location
    $lines | Select-String -Pattern '^test |test result|panicked|assert|^metric|^note' |
        ForEach-Object { Write-Host $_.Line }
    Write-Host "check headless_$test=$(if ($code -eq 0) { 'PASS' } else { 'FAIL' })"
}
