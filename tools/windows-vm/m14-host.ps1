# M14's host session (goal P): PotPlayer's and RVP's skin modes side by
# side, run only with the owner's permission after advance notice, every
# time. PotPlayer's window and then RVP's appear at 1280x720 for about half
# a minute each: idle, with a video, in fullscreen (a double click on the
# video, the pointer at the bottom) and with an audio file. Only the mouse
# is moved and clicked; nothing of the GPU's or the system's is changed.
#
# -Bundle is the portable RVP of the goal (rvp-p); -Data holds
# fixture-60s.mkv and fixture-tone.ogg (RVP's own test clips) and
# host-session.exe (spikes/host-session, built with cargo xwin). The
# captures go to -Out and are copied from there to
# ~/rvp-reference/m14 (never committed).
#
# PotPlayer keeps its window place, playlist and history: its registry key
# (HKCU\Software\DAUM\PotPlayerMini64) and playlist folder are saved
# before the session and written back after it; the values PotPlayer added
# during the session are listed (`new_value=`) to be removed by hand. RVP
# keeps its settings and state in -Out, not in the bundle.
param(
    [string]$Bundle = 'C:\Users\<user>\rvp-test\rvp-p',
    [string]$Data = 'C:\Users\<user>\rvp-test\m14-host\data',
    [string]$Out = 'C:\Users\<user>\rvp-test\m14-host\out',
    [string]$PotPlayer = 'C:\Program Files\DAUM\PotPlayer\PotPlayerMini64.exe',
    [string]$Skin = 'C:\Program Files\DAUM\PotPlayer\Skins\BlackSpace_en.dsf'
)
$key = 'HKCU\Software\DAUM\PotPlayerMini64'
$playlist = Join-Path $env:APPDATA 'PotPlayerMini64\Playlist'

if (Get-Process PotPlayerMini64 -ErrorAction SilentlyContinue) {
    Write-Host 'stop=PotPlayer is running; close it first'
    exit 2
}
Remove-Item -Recurse -Force $Out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Out | Out-Null

# PotPlayer's state before.
& reg.exe export $key "$Out\potplayer-before.reg" /y | Out-Null
if (Test-Path $playlist) {
    Copy-Item -Recurse $playlist "$Out\playlist-before"
}
$valuesBefore = @{}
foreach ($item in Get-ChildItem -Recurse "Registry::$key" -ErrorAction SilentlyContinue) {
    foreach ($name in $item.GetValueNames()) { $valuesBefore["$($item.Name)|$name"] = $true }
}
$root = Get-Item "Registry::$key" -ErrorAction SilentlyContinue
if ($root) { foreach ($name in $root.GetValueNames()) { $valuesBefore["$($root.Name)|$name"] = $true } }

& "$Data\host-session.exe" m14 --out $Out --potplayer $PotPlayer --rvp "$Bundle\rvp.exe" `
    --skin $Skin --video "$Data\fixture-60s.mkv" --audio "$Data\fixture-tone.ogg"
$code = $LASTEXITCODE

# PotPlayer's state written back.
& reg.exe export $key "$Out\potplayer-after.reg" /y | Out-Null
& reg.exe import "$Out\potplayer-before.reg" 2>&1 | Out-Null
Write-Host "registry_written_back=$LASTEXITCODE"
if (Test-Path "$Out\playlist-before") {
    # Copied over, never deleted first: the saved files win, a new one stays.
    Copy-Item -Recurse -Force "$Out\playlist-before\*" $playlist
    Write-Host "playlist_written_back=$(if ($?) { 0 } else { 1 })"
}
$items = @(Get-ChildItem -Recurse "Registry::$key" -ErrorAction SilentlyContinue)
if ($root) { $items += Get-Item "Registry::$key" }
foreach ($item in $items) {
    foreach ($name in $item.GetValueNames()) {
        if (-not $valuesBefore.ContainsKey("$($item.Name)|$name")) {
            Write-Host "new_value=$($item.Name) | $name"
        }
    }
}
Write-Host "host_session_exit=$code"
exit $code
