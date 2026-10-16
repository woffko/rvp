; RVP's Windows installer (M8): a per-user installation without
; administrator rights in %LOCALAPPDATA%\Programs\RVP, a Start menu entry,
; an uninstaller in Settings' app list, and RVP registered for the video
; and audio types of packaging/associations.tsv. RVP registers itself as a
; handler only: "Open with" and Default apps list it, and nothing becomes
; its default (owner decision 2026-10-16). The uninstaller removes what the
; installer wrote and nothing else: the files it installed, then folders
; left empty, and of shared registry keys only RVP's values.
;
; tools/packaging/package.py writes the two includes (types.nsh: the
; registry entries per type; files.nsh: the installed files) and runs
; makensis -DVERSION=... -DNUMERIC=a.b.c.d -DSOURCE=bundle -DOUTFILE=...
;          -DINCLUDES=dir rvp.nsi

Unicode true
SetCompressor /SOLID lzma
RequestExecutionLevel user
ManifestDPIAware true

!include "MUI2.nsh"
!include "FileFunc.nsh"

!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\RVP"
!define APP_KEY "Software\RVP"

Name "RVP"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\RVP"
InstallDirRegKey HKCU "${APP_KEY}" "InstallDir"
BrandingText "RVP ${VERSION}"

VIProductVersion "${NUMERIC}"
VIAddVersionKey "ProductName" "RVP"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "RVP installer"
VIAddVersionKey "FileVersion" "${NUMERIC}"
VIAddVersionKey "LegalCopyright" "GPL-3.0-or-later"

!define MUI_ICON "${SOURCE}\rvp.ico"
!define MUI_UNICON "${SOURCE}\rvp.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\rvp.exe"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Russian"

!include "${INCLUDES}\types.nsh"
!include "${INCLUDES}\files.nsh"

; Tells Explorer the registered types changed.
!macro NotifyShell
    System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend

Section "RVP"
    SetOutPath "$INSTDIR"
    File /r "${SOURCE}\*.*"
    WriteUninstaller "$INSTDIR\uninstall.exe"
    CreateShortcut "$SMPROGRAMS\RVP.lnk" "$INSTDIR\rvp.exe"

    WriteRegStr HKCU "${APP_KEY}" "InstallDir" "$INSTDIR"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "RVP"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "RVP"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\rvp.exe,0"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
    WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
    WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    WriteRegDWORD HKCU "${UNINSTALL_KEY}" "EstimatedSize" $0

    ; RVP as an application that opens these types, for "Open with" and
    ; Default apps; no type's default changes.
    WriteRegStr HKCU "${APP_KEY}\Capabilities" "ApplicationName" "RVP"
    WriteRegStr HKCU "${APP_KEY}\Capabilities" "ApplicationDescription" "A PotPlayer-style media player"
    WriteRegStr HKCU "${APP_KEY}\Capabilities" "ApplicationIcon" "$INSTDIR\rvp.exe,0"
    WriteRegStr HKCU "Software\Classes\Applications\rvp.exe" "FriendlyAppName" "RVP"
    WriteRegStr HKCU "Software\Classes\Applications\rvp.exe\DefaultIcon" "" "$INSTDIR\rvp.exe,0"
    WriteRegStr HKCU "Software\Classes\Applications\rvp.exe\shell\open\command" "" '"$INSTDIR\rvp.exe" "%1"'
    !insertmacro RegisterTypes
    WriteRegStr HKCU "Software\RegisteredApplications" "RVP" "${APP_KEY}\Capabilities"
    !insertmacro NotifyShell
SectionEnd

Section "Uninstall"
    ; RVP's value first: UnregisterTypes then removes the key if RVP
    ; created it and it is empty.
    DeleteRegValue HKCU "Software\RegisteredApplications" "RVP"
    !insertmacro UnregisterTypes
    DeleteRegKey HKCU "Software\Classes\Applications\rvp.exe"
    DeleteRegKey HKCU "${APP_KEY}"
    DeleteRegKey HKCU "${UNINSTALL_KEY}"
    Delete "$SMPROGRAMS\RVP.lnk"
    ; The installed files only: settings and state live in the user's
    ; profile, and stay; so does anything else in the folder.
    !insertmacro RemoveFiles
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"
    !insertmacro NotifyShell
SectionEnd
