param([string[]]$Extra = @())
# Runs the RVP bundle in this session (session 0 over SSH: software OpenGL,
# windows are never exposed). Settings and state stay in C:\rvp-test.
Set-Location C:\rvp-test\rvp-bundle
$env:QT_OPENGL = 'software'
$env:RVP_CONFIG_DIR = 'C:\rvp-test\config'
$env:RVP_STATE_DIR = 'C:\rvp-test\state'
& .\rvp.exe @Extra 2>&1 | ForEach-Object { "$_" }
"exit=$LASTEXITCODE hex=" + ('{0:X}' -f $LASTEXITCODE)
