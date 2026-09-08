; NexQ NSIS installer hooks
; Installs Visual C++ 2015-2022 Redistributable if not present

!macro NSIS_HOOK_PREINSTALL
  ; Check if Visual C++ 2015-2022 Redistributable (x64) is already installed
  ReadRegDWord $0 HKLM "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" "Installed"

  ${If} $0 == 1
    DetailPrint "Visual C++ 运行库：已安装"
    Goto vcredist_done
  ${EndIf}

  DetailPrint "正在下载 Visual C++ 运行库…"
  Delete "$TEMP\vc_redist.x64.exe"

  NSISdl::download "https://aka.ms/vs/17/release/vc_redist.x64.exe" "$TEMP\vc_redist.x64.exe"
  Pop $0

  ${If} $0 == "success"
    DetailPrint "正在安装 Visual C++ 运行库…"
    ExecWait '"$TEMP\vc_redist.x64.exe" /install /quiet /norestart' $1
    Delete "$TEMP\vc_redist.x64.exe"
  ${Else}
    MessageBox MB_ICONEXCLAMATION "无法下载 Visual C++ 运行库。$\n请从以下地址手动安装：$\nhttps://aka.ms/vs/17/release/vc_redist.x64.exe"
  ${EndIf}

  vcredist_done:
!macroend
