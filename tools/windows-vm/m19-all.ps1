# M19 checks on the Windows test VM (session 0 over SSH, software OpenGL,
# Windows in English): RVP's interface languages. Copy this file and
# run.ps1 to C:\rvp-test with the bundle in C:\rvp-test\rvp-bundle and
# fixture-60s.mkv, fixture-chapters.mkv and fixture-chapters.srt next to
# rvp.exe.
#
# For each translation (`-Languages`, by default every one RVP has):
# 1. M14's language steps alone (`--selftest=language --language=CODE`,
#    shell/qml/SelfTestM14.qml) on fresh settings: the language at the
#    first start is Windows' (`--expect-system-language=en`), the
#    translation loads by its own name, and every text of the settings
#    window fits its control.
# 2. The screenshot tour (`--selftest=tour --tour-grab`, the README's
#    views) with the interface in that language; each view's windows are
#    saved in C:\rvp-test\out\m19\tour-CODE (and English's in tour-en) to
#    be looked through for cut, overlapping or missing glyphs, Chinese,
#    Japanese and Korean in Windows' own fonts.
param(
    [string]$Exe = 'C:\rvp-test\rvp-bundle\rvp.exe',
    [string[]]$Languages = @('ru', 'pt_BR', 'de', 'es', 'fr', 'it', 'pl', 'uk', 'tr', 'zh_CN', 'zh_TW', 'ja', 'ko')
)
$bundle = Split-Path $Exe
$filter = 'DPI|DpiAware|^grabbed=|^qt\.|pw\.conf|ametadata|astats|Parsed_'
$out = 'C:\rvp-test\out\m19'
Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $out | Out-Null

function Fresh([string]$Settings) {
    Remove-Item -Recurse -Force C:\rvp-test\state, C:\rvp-test\config -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force C:\rvp-test\config | Out-Null
    Set-Content -Path C:\rvp-test\config\settings.json -Value $Settings -Encoding ascii
}

foreach ($code in $Languages) {
    Write-Host "== language $code"
    Fresh '{ "ipc-server": "" }'
    $extra = @('--selftest=language', "--language=$code", '--expect-system-language=en',
        "--out=$out\language-$code", '--mpv-options=ao=null', "$bundle\fixture-60s.mkv")
    & "$PSScriptRoot\run.ps1" -Exe $Exe -Extra $extra | Select-String -NotMatch $filter |
        Select-String -Pattern '^(check |failures=|exit=)' | ForEach-Object { Write-Host $_.Line }
}

foreach ($code in @('en') + $Languages) {
    Write-Host "== tour $code"
    Fresh "{ `"ipc-server`": `"`", `"general`": { `"language`": `"$code`" } }"
    $extra = @('--selftest=tour', '--tour-grab', "--out=$out\tour-$code", '--mpv-options=ao=null',
        "$bundle\fixture-60s.mkv", "$bundle\fixture-chapters.mkv")
    & "$PSScriptRoot\run.ps1" -Exe $Exe -Extra $extra | Select-String -NotMatch $filter |
        Select-String -Pattern '^(check |failures=|exit=|tour_shot=)' | ForEach-Object { Write-Host $_.Line }
    $grabs = @(Get-ChildItem "$out\tour-$code\*.png" -ErrorAction SilentlyContinue)
    Write-Host "grabs $code=$($grabs.Count)"
}
