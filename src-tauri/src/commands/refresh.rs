//! 后台自动刷新控制命令

use mqdesk_core::error::AppResult;
use mqdesk_core::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn set_refresh_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> AppResult<()> {
    state.refresh_task.set_enabled(enabled);
    Ok(())
}

#[tauri::command]
pub fn set_refresh_interval(ms: u64, state: State<'_, Arc<AppState>>) -> AppResult<()> {
    state.refresh_task.set_interval(ms);
    Ok(())
}
