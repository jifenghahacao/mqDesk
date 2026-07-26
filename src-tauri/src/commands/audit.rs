//! 队列审计日志命令

use mqdesk_core::chrono::Utc;
use mqdesk_core::error::{AppError, AppResult};
use mqdesk_core::models::{AuditFilter, QueueAuditLog};
use mqdesk_core::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn list_queue_audit_logs(
    filter: AuditFilter,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<QueueAuditLog>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let limit = filter.limit.unwrap_or(200);
    let logs = state
        .storage
        .list_audit_logs(filter.queue_name.as_deref(), filter.vhost.as_deref(), limit)?;
    Ok(logs)
}

#[tauri::command]
pub fn export_queue_audit_logs(
    filter: AuditFilter,
    path: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let limit = filter.limit.unwrap_or(2000);
    let logs = state
        .storage
        .list_audit_logs(filter.queue_name.as_deref(), filter.vhost.as_deref(), limit)?;
    let json = mqdesk_core::serde_json::to_string_pretty(&logs).map_err(|e| AppError::Storage(e.to_string()))?;
    std::fs::write(&path, json).map_err(|e| AppError::Storage(format!("导出失败：{e}")))?;
    Ok(())
}

/// 内部 helper：记录一条审计日志。供其他命令调用。
pub fn record(
    state: &AppState,
    action: &str,
    target_queue: &str,
    vhost: &str,
    detail: &str,
) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    let operator = whoami::username();
    let log = QueueAuditLog {
        id: format!("{}-{}-{}", target_queue, action, &now[..19]),
        timestamp: now,
        action: action.to_string(),
        target_queue: target_queue.to_string(),
        vhost: vhost.to_string(),
        detail: detail.to_string(),
        operator,
    };
    state.storage.append_audit_log(&log)?;
    Ok(())
}
