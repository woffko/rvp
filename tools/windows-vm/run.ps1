param([string[]]$Extra = @(), [string]$Exe = 'C:\rvp-test\rvp-bundle\rvp.exe')
# Runs the RVP bundle, or the copy `$Exe` names, in this session (session 0
# over SSH: software OpenGL, windows are never exposed). Settings and state
# stay in C:\rvp-test. The caller's folder is restored after (m8-all.ps1
# removes the installed copy's folder later).
Push-Location (Split-Path $Exe)
$env:QT_OPENGL = 'software'
$env:RVP_CONFIG_DIR = 'C:\rvp-test\config'
$env:RVP_STATE_DIR = 'C:\rvp-test\state'
& $Exe @Extra 2>&1 | ForEach-Object { "$_" }
"exit=$LASTEXITCODE hex=" + ('{0:X}' -f $LASTEXITCODE)
Pop-Location
