//! 集群节点监控命令

use mqdesk_core::error::{AppError, AppResult};
use mqdesk_core::models::NodeInfo;
use mqdesk_core::state::AppState;
use std::sync::Arc;
use tauri::State;

/// 列出当前活跃连接所在 RabbitMQ 集群的所有节点
#[tauri::command]
pub async fn list_nodes(state: State<'_, Arc<AppState>>) -> AppResult<Vec<NodeInfo>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let management = state.rabbit_management();
    management.list_nodes().await
}
