# M8: Windows' media controls (System Media Transport Controls) in a
# desktop session. Windows' media session manager runs as a per-user
# service (NPSMSvc) only for a user signed in at a desktop, which the test
# VM's SSH sessions are not; CI's hosted runners are, so CI's
# package-windows job runs this script. On the owner's host it runs only
# with the owner's permission, every time (AGENTS.md).
#
# It starts the portable RVP from this folder without sound (ao=null), on an
# instance channel of its own (it never reaches the owner's RVP) with SVP's
# pipe off, and an album of two copies of the test clip. Then it finds
# RVP's session in Windows' media session manager, the list the flyout and
# the media keys use, reads the title and the status, and presses Pause,
# Play, Next, Previous and Stop there. Last, it closes RVP's window with
# WM_CLOSE. No keys are injected and nothing is captured.
#
# RVP's session is the one whose app ID names rvp, so no other RVP may
# play meanwhile. Run with Windows PowerShell 5.1 (WinRT types), in a folder
# that holds the portable ZIP's RVP folder and fixture-60s.mkv:
#   powershell.exe -NoProfile -ExecutionPolicy Bypass -File m8-smtc.ps1
param([string]$Base = $PSScriptRoot)

$ErrorActionPreference = 'Stop'
$rvp = "$Base\RVP\rvp.exe"
$script:failures = 0

function Check([string]$Name, [bool]$Ok, [string]$Detail = '') {
    if ($Ok) { Write-Host "check $Name=PASS $Detail" } else { $script:failures++; Write-Host "check $Name=FAIL $Detail" }
}

# WinRT's asynchronous calls, waited for through .NET tasks.
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
    $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
} | Select-Object -First 1
function Await($Operation, [type]$Type) {
    $task = $asTask.MakeGenericMethod($Type).Invoke($null, @($Operation))
    if (-not $task.Wait(10000)) { throw 'a media session call timed out' }
    $task.Result
}
$null = [Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager, Windows.Media.Control, ContentType = WindowsRuntime]
$properties = [Windows.Media.Control.GlobalSystemMediaTransportControlsSessionMediaProperties]
# Without a desktop session (the VM's session 0) there is no session
# manager: stop before RVP starts.
try {
    $manager = Await ([Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager]::RequestAsync()) `
        ([Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager])
} catch {
    Check 'session_manager' $false $_.Exception.GetBaseException().Message
    Write-Host "failures=$script:failures"
    exit 1
}

# Closes a process's Qt windows as their close buttons would.
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class RvpWindows {
    delegate bool EnumProc(IntPtr window, IntPtr data);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc proc, IntPtr data);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr window, StringBuilder name, int size);
    [DllImport("user32.dll")] static extern bool PostMessage(IntPtr window, uint message, IntPtr w, IntPtr l);
    // Posts WM_CLOSE to the top-level windows of a process whose class is a
    // QWindow's (Qt6103QWindowOwnDCIcon for RVP's main window; not Qt's
    // hidden helpers); returns how many.
    public static int Close(int process) {
        int count = 0;
        EnumWindows(delegate (IntPtr window, IntPtr data) {
            uint owner;
            GetWindowThreadProcessId(window, out owner);
            var name = new StringBuilder(256);
            if (owner == (uint)process && GetClassName(window, name, name.Capacity) > 0 &&
                name.ToString().Contains("QWindow")) {
                PostMessage(window, 0x0010, IntPtr.Zero, IntPtr.Zero);
                count++;
            }
            return true;
        }, IntPtr.Zero);
        return count;
    }
}
'@

# RVP's session: the one whose app ID names rvp.exe.
function Session {
    $manager.GetSessions() | Where-Object { $_.SourceAppUserModelId -match 'rvp' } | Select-Object -First 1
}

function Title($Session) { (Await ($Session.TryGetMediaPropertiesAsync()) $properties).Title }

function Status($Session) { "$($Session.GetPlaybackInfo().PlaybackStatus)" }

# Waits up to 10 s until `$Test` holds for RVP's session; returns what the
# session shows then.
function WaitFor([scriptblock]$Test) {
    $shown = $null
    $deadline = (Get-Date).AddSeconds(10)
    do {
        # A session that goes away meanwhile is read again.
        try {
            $session = Session
            if ($session) {
                $shown = [pscustomobject]@{ Title = (Title $session); Status = (Status $session) }
                if (& $Test $shown) { return $shown }
            }
        } catch {}
        Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)
    $shown
}

$clips = @("$Base\clip-a.mkv", "$Base\clip-b.mkv")
foreach ($clip in $clips) { Copy-Item -Force "$Base\fixture-60s.mkv" $clip }
Set-Content -Path "$Base\RVP\settings.json" -Value '{ "ipc-server": "" }' -Encoding ascii
$process = Start-Process -PassThru -FilePath $rvp -ArgumentList (@('--instance=m8smtc', '--skin=', '--mpv-options=ao=null') + $clips)

try {
    $deadline = (Get-Date).AddSeconds(30)
    while (-not (Session) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 250 }
    $session = Session
    Check 'session' ($null -ne $session) "app ID '$($session.SourceAppUserModelId)'"
    if ($session) {
        $shown = WaitFor { param($s) $s.Status -eq 'Playing' -and $s.Title -match 'clip-a' }
        Check 'playing_title' ($shown.Status -eq 'Playing' -and $shown.Title -match 'clip-a') "$($shown.Status) '$($shown.Title)'"
        # Windows offers the button that fits the status: Pause while
        # playing, Play while paused.
        $controls = $session.GetPlaybackInfo().Controls
        Check 'buttons_playing' ($controls.IsPauseEnabled -and $controls.IsNextEnabled) `
            "pause $($controls.IsPauseEnabled) next $($controls.IsNextEnabled) previous $($controls.IsPreviousEnabled)"

        $null = Await ($session.TryPauseAsync()) ([bool])
        $shown = WaitFor { param($s) $s.Status -eq 'Paused' }
        Check 'pause' ($shown.Status -eq 'Paused') $shown.Status
        $controls = (Session).GetPlaybackInfo().Controls
        Check 'buttons_paused' ($controls.IsPlayEnabled) "play $($controls.IsPlayEnabled)"
        $null = Await ((Session).TryPlayAsync()) ([bool])
        $shown = WaitFor { param($s) $s.Status -eq 'Playing' }
        Check 'play' ($shown.Status -eq 'Playing') $shown.Status

        $null = Await ((Session).TrySkipNextAsync()) ([bool])
        $shown = WaitFor { param($s) $s.Title -match 'clip-b' }
        Check 'next' ($shown.Title -match 'clip-b') "'$($shown.Title)'"
        $null = Await ((Session).TrySkipPreviousAsync()) ([bool])
        $shown = WaitFor { param($s) $s.Title -match 'clip-a' }
        Check 'previous' ($shown.Title -match 'clip-a') "'$($shown.Title)'"

        $null = Await ((Session).TryStopAsync()) ([bool])
        $shown = WaitFor { param($s) $s.Status -eq 'Stopped' }
        Check 'stop' ($shown.Status -eq 'Stopped') $shown.Status
    }
} finally {
    $windows = [RvpWindows]::Close($process.Id)
    if (-not $process.WaitForExit(15000)) {
        Stop-Process -Id $process.Id -Force
        Check 'closed' $false "WM_CLOSE to $windows windows; stopped after 15 s"
    } else {
        Check 'closed' $true "WM_CLOSE to $windows windows, exit $($process.ExitCode)"
    }
    Remove-Item -Force $clips -ErrorAction SilentlyContinue
}
Write-Host "failures=$script:failures"
if ($script:failures -gt 0) { exit 1 }
