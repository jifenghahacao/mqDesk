//! 绑定命令

use crate::commands::audit;
use mqdesk_core::error::{AppError, AppResult};
use mqdesk_core::models::BindingInfo;
use mqdesk_core::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn list_queue_bindings(
    queue_name: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<BindingInfo>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let management = state.rabbit_management();
    management.list_queue_bindings(&queue_name).await
}

#[tauri::command]
pub async fn delete_queue_binding(
    queue_name: String,
    binding: BindingInfo,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let vhost = state
        .get_active()
        .map(|c| c.vhost)
        .unwrap_or_else(|| binding.vhost.clone());
    let management = state.rabbit_management();
    management.delete_binding(&binding).await?;
    let detail = format!("source={}, routing_key={}", binding.source, binding.routing_key);
    let _ = audit::record(&**state, "delete_binding", &queue_name, &vhost, &detail);
    Ok(())
}
