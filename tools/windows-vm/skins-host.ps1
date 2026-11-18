# The skin check's host session (2026-11-04): every skin of PotPlayer's
# Skins folder in PotPlayer and then in RVP, switched in the running player
# while a video plays (`host-session skins`). Run only with the owner's
# permission after advance notice, every time. PotPlayer's window and then
# RVP's appear at 1280x720, go to fullscreen and back for each skin: about
# 15 s a skin and player. Only the mouse is moved and clicked.
#
# -Bundle is the portable RVP to check; -Video a local 16:9 clip;
# host-session.exe (spikes/host-session, built with cargo xwin) lies in
# -Tools. The captures and session.log go to -Out and are copied from there
# to ~/rvp-reference (never committed). -Only names skins by file
# stem (comma separated); -Players potplayer or rvp runs one player.
#
# PotPlayer takes a skin it opens, or, when it does not, is started again
# with its LastSkinName set: its registry key (HKCU\Software\DAUM\
# PotPlayerMini64) and playlist folder are saved before the session and
# written back after it, as m14-host.ps1 does; the values PotPlayer added
# are listed (`new_value=`) to be removed by hand. RVP keeps its settings,
# state and log in -Out.
param(
    [string]$Bundle = "$env:USERPROFILE\rvp-test\rvp-s",
    [string]$Video = "$env:USERPROFILE\rvp-test\clips\video-1080p\video-1080p23976-h264.mkv",
    [string]$Tools = "$env:USERPROFILE\rvp-test\skins-host",
    [string]$Out = "$env:USERPROFILE\rvp-test\skins-host\out",
    [string]$PotPlayer = 'C:\Program Files\DAUM\PotPlayer\PotPlayerMini64.exe',
    [string]$Skins = 'C:\Program Files\DAUM\PotPlayer\Skins',
    [string]$Players = 'potplayer,rvp',
    [string]$Only = ''
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
$root = Get-Item "Registry::$key" -ErrorAction SilentlyContinue
$items = @(Get-ChildItem -Recurse "Registry::$key" -ErrorAction SilentlyContinue)
if ($root) { $items += $root }
foreach ($item in $items) {
    foreach ($name in $item.GetValueNames()) { $valuesBefore["$($item.Name)|$name"] = $true }
}

$arguments = @('skins', '--players', $Players, '--out', $Out, '--potplayer', $PotPlayer,
    '--rvp', "$Bundle\rvp.exe", '--skins', $Skins, '--video', $Video)
if ($Only -ne '') { $arguments += @('--only', $Only) }
& "$Tools\host-session.exe" @arguments
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
