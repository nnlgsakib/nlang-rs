Unicode true
Name "Nlang"
OutFile "NlangSetup.exe"
InstallDir "$LocalAppData\Nlang"
RequestExecutionLevel user
!include "WinMessages.nsh"
Var GCCPath
Var InstalledGCC
Var PrevUserPath

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "Install"
  SetOutPath "$InstDir\bin"
  File /nonfatal /a /r "D:\projects\rust\nlang\bin\*.exe"
  nsExec::ExecToStack 'cmd /C gcc --version'
  Pop $0
  Pop $1
  StrCmp $0 0 +7 0
  SetOutPath "$InstDir\gcc"
  File /r "D:\projects\rust\nlang\gcc\*"
  StrCpy $GCCPath "$InstDir\gcc\bin\gcc.exe"
  StrCpy $InstalledGCC 1
  Goto +3
  StrCpy $GCCPath "gcc"
  StrCpy $InstalledGCC 0
  WriteRegStr HKCU "Environment" "NLANG_GCC" "$GCCPath"
  System::Call 'Kernel32::SetEnvironmentVariableW(w "NLANG_GCC", w "$GCCPath")'
  System::Call 'USER32::SendMessageTimeoutW(p 0xFFFF, i ${WM_SETTINGCHANGE}, i 0, t "Environment", i 0, i 1000, *i .r0)'
  ; Add InstallDir\bin to PATH for running nlang from any shell
  ReadRegStr $PrevUserPath HKCU "Environment" "Path"
  StrCmp $PrevUserPath "" 0 +4
  WriteRegExpandStr HKCU "Environment" "Path" "$InstDir\bin"
  System::Call 'Kernel32::SetEnvironmentVariableW(w "Path", w "$InstDir\\bin")'
  Goto +6
  StrCpy $2 "$PrevUserPath;$InstDir\bin"
  WriteRegExpandStr HKCU "Environment" "Path" "$2"
  System::Call 'Kernel32::SetEnvironmentVariableW(w "Path", w "$2")'
  System::Call 'USER32::SendMessageTimeoutW(p 0xFFFF, i ${WM_SETTINGCHANGE}, i 0, t "Environment", i 0, i 1000, *i .r0)'
  WriteRegStr HKCU "Software\Nlang" "InstalledGCC" "$InstalledGCC"
  WriteRegStr HKCU "Software\Nlang" "InstallDir" "$InstDir"
  WriteUninstaller "$InstDir\Uninstall.exe"
SectionEnd

Section "Uninstall"
  Delete "$InstDir\bin\*.exe"
  RMDir /r "$InstDir\bin"
  ReadRegStr $0 HKCU "Software\Nlang" "InstalledGCC"
  StrCmp $0 1 0 +4
  RMDir /r "$InstDir\gcc"
  DeleteRegValue HKCU "Environment" "NLANG_GCC"
  System::Call 'USER32::SendMessageTimeoutW(p 0xFFFF, i ${WM_SETTINGCHANGE}, i 0, t "Environment", i 0, i 1000, *i .r0)'
  DeleteRegKey HKCU "Software\Nlang"
  RMDir /r "$InstDir"
SectionEnd