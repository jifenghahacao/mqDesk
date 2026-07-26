; NSIS uninstall hook: 清理 MQDesk 本地 sled 数据库目录
; Tauri 默认只删除 $APPDATA\${BUNDLEID}，但本项目使用 $APPDATA\mqdesk

!macro NSIS_HOOK_POSTUNINSTALL
  ; 仅在用户勾选"删除应用数据"且不是更新模式时执行
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    RmDir /r "$APPDATA\mqdesk"
  ${EndIf}
!macroend
