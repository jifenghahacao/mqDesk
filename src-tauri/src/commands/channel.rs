//! 信道命令

use mqdesk_core::error::{AppError, AppResult};
use mqdesk_core::models::{ChannelInfo, Paginated, Pagination};
use mqdesk_core::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn list_channels(
    connection_name: Option<String>,
    pagination: Option<Pagination>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Paginated<ChannelInfo>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let pagination = pagination.unwrap_or_default();
    let management = state.rabbit_management();
    management
        .list_channels(connection_name.as_deref(), &pagination)
        .await
}
