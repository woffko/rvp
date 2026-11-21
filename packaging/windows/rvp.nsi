; RVP's Windows installer (M8): a per-user installation without
; administrator rights in %LOCALAPPDATA%\Programs\RVP, a Start menu entry,
; an uninstaller in Settings' app list, and RVP registered for the video
; and audio types of packaging/associations.tsv. RVP registers itself as a
; handler only: "Open with" and Default apps list it, and nothing becomes
; its default (owner decision 2026-10-16). The uninstaller removes what the
; installer wrote and nothing else: the files it installed, then folders
; left empty, and of shared registry keys only RVP's values. It also
; removes what RVP registered itself from its settings
; (crates/app/src/registration.rs, the Run value, the screen saver), each
; only while it still names this installation's rvp.exe: another copy of
; RVP may have registered over it.
;
; When Unified Remote Server is installed, a component copies RVP's remote
; for it (M15) into its custom remotes folder
; (%ProgramData%\Unified Remote\Remotes\Custom\RVP); the uninstaller
; removes the files it copied there, and the folder when it is empty. The
; remote's settings.prop, which Unified Remote's manager edits, is not
; written over. Other users of the computer share that folder: the remote
; goes only into an RVP folder this user's installation made (noted under
; RVP's key) or makes now, and file work there never goes through a
; junction or a link.
;
; tools/packaging/package.py writes the includes (types.nsh: the registry
; entries per type; files.nsh: the installed files; remote.nsh: the
; remote's files) and runs
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
; The components page shows only when there is a choice: Unified Remote
; is installed.
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipComponents
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
; RVP's other languages and their LangStrings (shell/i18n, package.py).
!include "${INCLUDES}\languages.nsh"

!include "${INCLUDES}\types.nsh"
!include "${INCLUDES}\files.nsh"
!include "${INCLUDES}\remote.nsh"

LangString REMOTE_SECTION ${LANG_ENGLISH} "Remote for Unified Remote"

; Unified Remote's custom remotes folder.
Var RemoteDir

; Tells Explorer the registered types changed.
!macro NotifyShell
    System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend

; Jumps to `Label` when `Path` is missing or a reparse point (a junction or
; a link: file work through it lands elsewhere). Uses $R9.
!macro IfLinkOrMissing Path Label
    System::Call 'kernel32::GetFileAttributesW(w "${Path}") i .R9'
    ; INVALID_FILE_ATTRIBUTES has every bit, FILE_ATTRIBUTE_REPARSE_POINT too.
    IntOp $R9 $R9 & 0x400
    IntCmp $R9 0 0 ${Label} ${Label}
!macroend

; Jumps to `Label` when the command `Command` starts this installation's
; rvp.exe (`"$INSTDIR\rvp.exe" ...`, as RVP writes its commands). Uses $R8
; and $R9.
!macro IfOurs Command Label
    StrLen $R8 '"$INSTDIR\rvp.exe"'
    StrCpy $R9 "${Command}" $R8
    StrCmp $R9 '"$INSTDIR\rvp.exe"' ${Label}
!macroend

; An AutoPlay handler RVP registered (registration::AUTOPLAY), with its
; event's entry, while its icon is this installation's rvp.exe.
!define HANDLERS "Software\Microsoft\Windows\CurrentVersion\Explorer\AutoplayHandlers"
!macro RemoveHandler Event
    ReadRegStr $0 HKCU "${HANDLERS}\Handlers\RVP${Event}" "DefaultIcon"
    StrCmp $0 "$INSTDIR\rvp.exe,0" 0 +3
        DeleteRegKey HKCU "${HANDLERS}\Handlers\RVP${Event}"
        DeleteRegValue HKCU "${HANDLERS}\EventHandlers\${Event}" "RVP${Event}"
!macroend

Section "RVP" SecMain
    SectionIn RO
    SetOutPath "$INSTDIR"
    File /r "${SOURCE}\*.*"
    WriteUninstaller "$INSTDIR\uninstall.exe"
    CreateShortcut "$SMPROGRAMS\RVP.lnk" "$INSTDIR\rvp.exe"

    WriteRegStr HKCU "${APP_KEY}" "InstallDir" "$INSTDIR"
    ; The uninstaller speaks the installer's language (un.onInit).
    WriteRegStr HKCU "${APP_KEY}" "InstallerLanguage" "$LANGUAGE"
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
    WriteRegStr HKCU "Software\Classes\Applications\rvp.exe\shell\open\command" "" '"$INSTDIR\rvp.exe" -- "%1"'
    !insertmacro RegisterTypes
    WriteRegStr HKCU "Software\RegisteredApplications" "RVP" "${APP_KEY}\Capabilities"
    !insertmacro NotifyShell
SectionEnd

Section "$(REMOTE_SECTION)" SecRemote
    ; An RVP folder there that this user's installation did not note is
    ; someone else's.
    IfFileExists "$RemoteDir\RVP\*.*" 0 remote_create
        ReadRegStr $0 HKCU "${APP_KEY}" "UnifiedRemote"
        StrCmp $0 "$RemoteDir\RVP" remote_create
        DetailPrint "$RemoteDir\RVP is not this installation's: the remote was not copied."
        Goto remote_done
    remote_create:
    CreateDirectory "$RemoteDir\RVP"
    !insertmacro IfLinkOrMissing "$RemoteDir" remote_link
    !insertmacro IfLinkOrMissing "$RemoteDir\RVP" remote_link
    ; A folder the user cannot write to is left alone.
    ClearErrors
    FileOpen $1 "$RemoteDir\RVP\.rvp-check" w
    IfErrors 0 writable
        DetailPrint "Cannot write to $RemoteDir\RVP: the remote was not copied."
        Goto remote_done
    remote_link:
        DetailPrint "$RemoteDir\RVP is a link or missing: the remote was not copied."
        Goto remote_done
    writable:
    FileClose $1
    Delete "$RemoteDir\RVP\.rvp-check"
    SetOutPath "$RemoteDir\RVP"
    !insertmacro InstallRemote
    WriteRegStr HKCU "${APP_KEY}" "UnifiedRemote" "$RemoteDir\RVP"
    SetOutPath "$INSTDIR"
    remote_done:
SectionEnd

Function .onInit
    ; Windows' language, or one the command line names by its LANGID
    ; (`/LANG=1041`: the test VM runs the installer in each language).
    ${GetParameters} $1
    ${GetOptions} $1 "/LANG=" $2
    StrCmp $2 "" +2
        StrCpy $LANGUAGE $2
    ReadEnvStr $0 PROGRAMDATA
    StrCpy $RemoteDir "$0\Unified Remote\Remotes\Custom"
    IfFileExists "$0\Unified Remote\Remotes\*.*" remote_found
        ; No Unified Remote: the component is off and hidden.
        SectionSetFlags ${SecRemote} 0
        SectionSetText ${SecRemote} ""
    remote_found:
FunctionEnd

Function SkipComponents
    SectionGetText ${SecRemote} $0
    StrCmp $0 "" 0 +2
        Abort
FunctionEnd

Function un.onInit
    ReadRegStr $0 HKCU "${APP_KEY}" "InstallerLanguage"
    StrCmp $0 "" +2
        StrCpy $LANGUAGE $0
FunctionEnd

; What RVP registered from its settings, each while it names this
; installation. Before the files go: the screen saver's short path needs
; rvp.scr.
Function un.RemoveOwnRegistration
    ; General > Automatically run at system startup.
    ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "RVP"
    !insertmacro IfOurs $0 run_ours
    Goto run_done
    run_ours:
        DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "RVP"
    run_done:
    ; Screen Saver > Set up: rvp.scr by its short path (or its long one).
    ReadRegStr $0 HKCU "Control Panel\Desktop" "SCRNSAVE.EXE"
    StrCmp $0 "" saver_done
    StrCmp $0 "$INSTDIR\rvp.scr" saver_ours
    GetFullPathName /SHORT $1 "$INSTDIR\rvp.scr"
    StrCmp $1 "" saver_done
    StrCmp $0 $1 saver_ours saver_done
    saver_ours:
        DeleteRegValue HKCU "Control Panel\Desktop" "SCRNSAVE.EXE"
    saver_done:
    ; Association > Play with RVP on folders.
    ReadRegStr $0 HKCU "Software\Classes\Directory\shell\RVP.play\command" ""
    !insertmacro IfOurs $0 folder_ours
    Goto folder_done
    folder_ours:
        DeleteRegKey HKCU "Software\Classes\Directory\shell\RVP.play"
    folder_done:
    ; Association > AutoPlay: the handlers' ProgID and each handler.
    ReadRegStr $0 HKCU "Software\Classes\RVP.AutoPlay\shell\play\command" ""
    !insertmacro IfOurs $0 autoplay_ours
    Goto autoplay_done
    autoplay_ours:
        DeleteRegKey HKCU "Software\Classes\RVP.AutoPlay"
    autoplay_done:
    !insertmacro RemoveHandler PlayVideoFilesOnArrival
    !insertmacro RemoveHandler PlayMusicFilesOnArrival
    !insertmacro RemoveHandler PlayCDAudioOnArrival
    !insertmacro RemoveHandler PlayVideoCDMovieOnArrival
    !insertmacro RemoveHandler PlaySuperVideoCDMovieOnArrival
    !insertmacro RemoveHandler PlayDVDMovieOnArrival
    !insertmacro RemoveHandler PlayBluRayOnArrival
    ; Every type's ProgID RVP.<ext> (the table's and the user's own), its
    ; "Open with" entry and its "Play with RVP" verb.
    StrCpy $2 0
    types_next:
        ClearErrors
        EnumRegKey $1 HKCU "Software\Classes" $2
        IfErrors types_done
        StrCmp $1 "" types_done
        StrCpy $3 $1 4
        StrCmp $3 "RVP." 0 types_skip
        ReadRegStr $0 HKCU "Software\Classes\$1\shell\open\command" ""
        !insertmacro IfOurs $0 types_ours
        Goto types_skip
        types_ours:
            StrCpy $3 $1 "" 4
            DeleteRegKey HKCU "Software\Classes\$1"
            DeleteRegValue HKCU "Software\Classes\.$3\OpenWithProgids" "$1"
            ReadRegStr $0 HKCU "Software\Classes\SystemFileAssociations\.$3\shell\RVP.play\command" ""
            !insertmacro IfOurs $0 verb_ours
            Goto types_next
            verb_ours:
                DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.$3\shell\RVP.play"
            ; The next key took this one's place in the list.
            Goto types_next
        types_skip:
        IntOp $2 $2 + 1
        Goto types_next
    types_done:
FunctionEnd

Section "Uninstall"
    ; The remote's files first: where they went is under RVP's key. Never
    ; through a junction or a link put in its place.
    ReadRegStr $0 HKCU "${APP_KEY}" "UnifiedRemote"
    StrCmp $0 "" no_remote
    ${GetParent} $0 $1
    !insertmacro IfLinkOrMissing $1 no_remote
    !insertmacro IfLinkOrMissing $0 no_remote
        !insertmacro RemoveRemote
    no_remote:
    Call un.RemoveOwnRegistration
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
