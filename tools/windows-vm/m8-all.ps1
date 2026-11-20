# M8 checks on the Windows test VM (session 0 over SSH, software OpenGL):
# the installer and the portable ZIP.
#
# The installer runs silently for this user: its files, the Start menu entry,
# the uninstall entry and RVP's registration for the media types ("Open
# with", Default apps), with no type's default changed. The installed copy
# passes the functional self-test and m7-all.ps1's checks (RVP Default,
# then BlackSpace) and writes crash logs (a panic, and a native crash with a
# minidump). The uninstaller then removes all of it:
# every registry key the installer touched reads as it did before, and the
# folder is gone. Then the installer installs and uninstalls in each of
# its languages (`/LANG=LANGID`, kept for the uninstaller). Last, the
# portable ZIP runs its self-test with its settings next to rvp.exe.
#
# Layout: this file, m7-all.ps1 and run.ps1 in C:\rvp-test; RVP-*-setup.exe,
# RVP-*-portable.zip and associations.tsv in C:\rvp-test\m8; the fixtures
# in C:\rvp-test\rvp-bundle (m7-all.ps1).
$m8 = 'C:\rvp-test\m8'
$fixtures = @('C:\rvp-test\rvp-bundle\fixture-60s.mkv', 'C:\rvp-test\rvp-bundle\fixture-3s.mkv')
$setup = Get-ChildItem "$m8\RVP-*-setup.exe" | Select-Object -First 1
$zip = Get-ChildItem "$m8\RVP-*-portable.zip" | Select-Object -First 1
$dir = "$env:LOCALAPPDATA\Programs\RVP"
$shortcut = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\RVP.lnk"
$filter = 'DPI|DpiAware|^grabbed=|^qt\.'
$script:failures = 0

function Check([string]$Name, [bool]$Ok, [string]$Detail = '') {
    if ($Ok) { Write-Host "check $Name=PASS $Detail" } else { $script:failures++; Write-Host "check $Name=FAIL $Detail" }
}

# A registry key with its values and subkeys as one text, or "absent".
function KeyText([string]$Path) {
    if (-not (Test-Path "Registry::$Path")) { return 'absent' }
    $lines = @()
    $key = Get-Item "Registry::$Path"
    foreach ($name in ($key.GetValueNames() | Sort-Object)) {
        $lines += "$Path :: $name = $($key.GetValue($name, $null, 'DoNotExpandEnvironmentNames'))"
    }
    foreach ($sub in (Get-ChildItem "Registry::$Path" | Sort-Object Name)) {
        $lines += KeyText $sub.Name
    }
    if ($lines.Count -eq 0) { $lines += "$Path (empty)" }
    return ($lines -join "`n")
}

# Every key the installer may write, in one text.
$types = (Get-Content "$m8\associations.tsv" | Where-Object { $_ -and -not $_.StartsWith('#') } |
    ForEach-Object { ($_ -split "`t")[0] })
function Snapshot {
    $keys = @('HKEY_CURRENT_USER\Software\Classes\Applications\rvp.exe', 'HKEY_CURRENT_USER\Software\RVP',
        'HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\RVP')
    foreach ($type in $types) {
        $keys += "HKEY_CURRENT_USER\Software\Classes\.$type"
        $keys += "HKEY_CURRENT_USER\Software\Classes\RVP.$type"
    }
    $text = ($keys | ForEach-Object { KeyText $_ }) -join "`n"
    $apps = Get-ItemProperty 'HKCU:\Software\RegisteredApplications' -ErrorAction SilentlyContinue
    return "$text`nRegisteredApplications RVP = $($apps.RVP)"
}

# The default program of a type as Explorer resolves it.
function Defaults {
    ($types | ForEach-Object {
        $choice = (Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.$_\UserChoice" -ErrorAction SilentlyContinue).ProgId
        $class = (Get-ItemProperty "Registry::HKEY_CLASSES_ROOT\.$_" -ErrorAction SilentlyContinue).'(default)'
        ".$_ $choice $class"
    }) -join "`n"
}

# Runs RVP with settings and state in `$Base`; returns its output.
function RunRvp([string]$Exe, [string[]]$Arguments, [string]$Base, [bool]$Portable = $false) {
    $env:QT_OPENGL = 'software'
    if ($Portable) {
        Remove-Item Env:RVP_CONFIG_DIR, Env:RVP_STATE_DIR -ErrorAction SilentlyContinue
    } else {
        $env:RVP_CONFIG_DIR = "$Base\config"
        $env:RVP_STATE_DIR = "$Base\state"
    }
    & $Exe @Arguments 2>&1 | ForEach-Object { "$_" }
    "exit=$LASTEXITCODE hex=" + ('{0:X}' -f $LASTEXITCODE)
}

function SelfTest([string]$Name, [string]$Exe, [string]$Base, [bool]$Portable = $false) {
    $out = "$m8\out-$Name"
    Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force $out | Out-Null
    $output = RunRvp $Exe (@('--selftest', "--out=$out", '--mpv-options=ao=null') + $fixtures) $Base $Portable
    $output | Select-String -NotMatch $filter | ForEach-Object { Write-Host "  $($_.Line)" }
    $result = ($output | Select-String '^failures=(\d+)' | Select-Object -Last 1)
    Check "selftest_$Name" ($null -ne $result -and $result.Matches[0].Groups[1].Value -eq '0') "$result"
}

Write-Host "== install $($setup.Name)"
# Keys another program made, which the uninstaller must leave as they are:
# an empty type key, and another player in a type's "Open with" list.
New-Item -Force 'HKCU:\Software\Classes\.m4b' | Out-Null
New-Item -Force 'HKCU:\Software\Classes\.mka\OpenWithProgids' | Out-Null
New-ItemProperty -Force -Path 'HKCU:\Software\Classes\.mka\OpenWithProgids' -Name 'OtherPlayer.mka' -Value '' | Out-Null
$before = Snapshot
$defaults = Defaults
Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
Start-Process -Wait -FilePath $setup.FullName -ArgumentList '/S'
Check 'installed_files' ((Test-Path "$dir\rvp.exe") -and (Test-Path "$dir\libmpv-2.dll") -and
    (Test-Path "$dir\Qt6Core.dll") -and (Test-Path "$dir\uninstall.exe")) $dir
Check 'start_menu_entry' (Test-Path $shortcut) $shortcut
$progids = (Get-Item 'HKCU:\Software\Classes\.mkv\OpenWithProgids' -ErrorAction SilentlyContinue).Property
Check 'open_with_mkv' ($progids -contains 'RVP.mkv') ($progids -join ',')
$command = (Get-ItemProperty 'HKCU:\Software\Classes\RVP.mkv\shell\open\command' -ErrorAction SilentlyContinue).'(default)'
Check 'command_mkv' ($command -eq "`"$dir\rvp.exe`" -- `"%1`"") $command
Check 'default_apps_entry' ((Get-ItemProperty 'HKCU:\Software\RegisteredApplications').RVP -eq 'Software\RVP\Capabilities')
$capability = (Get-ItemProperty 'HKCU:\Software\RVP\Capabilities\FileAssociations' -ErrorAction SilentlyContinue).'.mp3'
Check 'capabilities_mp3' ($capability -eq 'RVP.mp3') $capability
$entry = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\RVP' -ErrorAction SilentlyContinue
Check 'uninstall_entry' ($entry.DisplayName -eq 'RVP' -and $entry.QuietUninstallString) "version $($entry.DisplayVersion)"
Check 'defaults_unchanged' ((Defaults) -eq $defaults) 'UserChoice and HKCR defaults of every type'

Write-Host '== the installed copy'
$installedBase = "$m8\home-installed"
Remove-Item -Recurse -Force $installedBase -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$installedBase\config" | Out-Null
Set-Content -Path "$installedBase\config\settings.json" -Value '{ "ipc-server": "" }' -Encoding ascii
SelfTest 'installed' "$dir\rvp.exe" $installedBase
Write-Host '== m7-all.ps1 on the installed copy'
$m7 = & "$PSScriptRoot\m7-all.ps1" -Exe "$dir\rvp.exe" 6>&1 | ForEach-Object { "$_" }
$m7 | ForEach-Object { Write-Host "  $_" }
$m7Results = @($m7 | Select-String '^failures=(\d+)')
$m7Failed = @($m7Results | Where-Object { $_.Matches[0].Groups[1].Value -ne '0' })
Check 'm7_installed' ($m7Results.Count -eq 2 -and $m7Failed.Count -eq 0 -and ($m7 -match '^exe_icon=')) `
    "$(($m7Results | ForEach-Object { $_.Line }) -join ', ')"

Write-Host '== crash logs'
# --crash-test is a test hook: RVP takes it only with RVP_TEST_HOOKS=1.
$env:RVP_TEST_HOOKS = '1'
foreach ($kind in @('panic', 'native')) {
    $crashHome = "$m8\home-crash-$kind"
    Remove-Item -Recurse -Force $crashHome -ErrorAction SilentlyContinue
    $output = RunRvp "$dir\rvp.exe" @("--crash-test=$kind") $crashHome
    $logs = @(Get-ChildItem "$crashHome\state\crash-*.log" -ErrorAction SilentlyContinue)
    $text = ($logs | ForEach-Object { Get-Content $_.FullName -Raw }) -join ' '
    if ($kind -eq 'panic') {
        Check 'crash_log_panic' ($text -match "panic in thread 'main'" -and $text -match 'crash test') "$($logs.Name) $($output | Select-Object -Last 1)"
    } else {
        $dumps = @(Get-ChildItem "$crashHome\state\crash-*-native.dmp" -ErrorAction SilentlyContinue)
        Check 'crash_log_native' ($text -match 'unhandled exception 0xc0000005') "$($logs.Name) $($output | Select-Object -Last 1)"
        Check 'crash_minidump' ($dumps.Count -eq 1 -and $dumps[0].Length -gt 1000) "$($dumps.Name) $($dumps[0].Length) bytes"
    }
}
Remove-Item Env:RVP_TEST_HOOKS -ErrorAction SilentlyContinue

Write-Host '== uninstall'
Start-Process -Wait -FilePath "$dir\uninstall.exe" -ArgumentList '/S'
# The uninstaller copies itself to %TEMP% and returns at once.
$deadline = (Get-Date).AddSeconds(120)
while ((Test-Path $dir) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 500 }
Check 'folder_removed' (-not (Test-Path $dir)) $dir
Check 'start_menu_removed' (-not (Test-Path $shortcut))
$after = Snapshot
Check 'registry_as_before' ($after -eq $before) 'every key the installer touched'
if ($after -ne $before) {
    Compare-Object ($before -split "`n") ($after -split "`n") | ForEach-Object { Write-Host "  $($_.SideIndicator) $($_.InputObject)" }
}
Check 'defaults_still_unchanged' ((Defaults) -eq $defaults)
Check 'other_programs_kept' ((Test-Path 'HKCU:\Software\Classes\.m4b') -and
    ((Get-Item 'HKCU:\Software\Classes\.mka\OpenWithProgids').Property -contains 'OtherPlayer.mka'))
Remove-Item -Recurse -Force 'HKCU:\Software\Classes\.m4b', 'HKCU:\Software\Classes\.mka' -ErrorAction SilentlyContinue

# The installer in each of its languages (M19): Windows' language here is
# English, so the LANGID comes from the command line; the installer keeps
# it for the uninstaller, which removes it with RVP's key.
$baseline = Snapshot
$languages = [ordered]@{
    'ru' = 1049; 'pt_BR' = 1046; 'de' = 1031; 'es' = 1034; 'fr' = 1036; 'it' = 1040; 'pl' = 1045
    'uk' = 1058; 'tr' = 1055; 'zh_CN' = 2052; 'zh_TW' = 1028; 'ja' = 1041; 'ko' = 1042
}
foreach ($code in $languages.Keys) {
    $id = $languages[$code]
    Write-Host "== the installer in $code ($id)"
    Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
    Start-Process -Wait -FilePath $setup.FullName -ArgumentList @('/S', "/LANG=$id")
    $kept = (Get-ItemProperty 'HKCU:\Software\RVP' -ErrorAction SilentlyContinue).InstallerLanguage
    Check "installer_$code" ((Test-Path "$dir\rvp.exe") -and (Test-Path "$dir\uninstall.exe") -and $kept -eq "$id") `
        "language $kept"
    Start-Process -Wait -FilePath "$dir\uninstall.exe" -ArgumentList '/S'
    $deadline = (Get-Date).AddSeconds(120)
    while ((Test-Path $dir) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 500 }
    Check "uninstaller_$code" ((-not (Test-Path $dir)) -and (-not (Test-Path 'HKCU:\Software\RVP')) -and
        ((Snapshot) -eq $baseline)) $dir
}

Write-Host "== portable $($zip.Name)"
$portable = "$m8\portable"
Remove-Item -Recurse -Force $portable -ErrorAction SilentlyContinue
Expand-Archive -Path $zip.FullName -DestinationPath $portable
Check 'portable_marker' (Test-Path "$portable\RVP\rvp.portable")
Set-Content -Path "$portable\RVP\settings.json" -Value '{ "ipc-server": "" }' -Encoding ascii
SelfTest 'portable' "$portable\RVP\rvp.exe" $portable $true
Check 'portable_state_beside' (Test-Path "$portable\RVP\state") "$portable\RVP"
Remove-Item -Recurse -Force $portable -ErrorAction SilentlyContinue

Write-Host "failures=$script:failures"
