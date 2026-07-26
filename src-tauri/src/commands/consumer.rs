//! 消费者命令

use mqdesk_core::error::{AppError, AppResult};
use mqdesk_core::models::{ConsumerInfo, ConsumerMessage, ManualConsumer, ManualConsumerConfig};
use mqdesk_core::state::AppState;
use std::sync::Arc;
use tauri::State;

/// 列出当前活跃连接所在 vhost 的所有消费者（监控）
#[tauri::command]
pub async fn list_consumers(state: State<'_, Arc<AppState>>) -> AppResult<Vec<ConsumerInfo>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let management = state.rabbit_management();
    management.list_consumers().await
}

/// 创建手动消费者（消费者工作室）
#[tauri::command]
pub fn create_manual_consumer(
    config: ManualConsumerConfig,
    state: State<'_, Arc<AppState>>,
) -> AppResult<ManualConsumer> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    state.consumer_manager.create(config)
}

/// 启动手动消费者
#[tauri::command]
pub async fn start_manual_consumer(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<ManualConsumer> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let amqp_url = state.amqp_url()?;
    state.consumer_manager.start(&id, &amqp_url).await
}

/// 暂停手动消费者
#[tauri::command]
pub async fn pause_manual_consumer(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<ManualConsumer> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    state.consumer_manager.pause(&id).await
}

/// 继续手动消费者
#[tauri::command]
pub async fn resume_manual_consumer(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<ManualConsumer> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let amqp_url = state.amqp_url()?;
    state.consumer_manager.resume(&id, &amqp_url).await
}

/// 销毁手动消费者
#[tauri::command]
pub async fn destroy_manual_consumer(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    state.consumer_manager.destroy(&id).await
}

/// 获取单个手动消费者状态
#[tauri::command]
pub fn get_manual_consumer(id: String, state: State<'_, Arc<AppState>>) -> AppResult<ManualConsumer> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    state.consumer_manager.get(&id)
}

/// 列出所有手动消费者
#[tauri::command]
pub fn list_manual_consumers(state: State<'_, Arc<AppState>>) -> AppResult<Vec<ManualConsumer>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    Ok(state.consumer_manager.list())
}

/// 列出某消费者最近 N 条消息
#[tauri::command]
pub fn list_manual_consumer_messages(
    id: String,
    limit: Option<usize>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<ConsumerMessage>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    state.consumer_manager.list_messages(&id, limit.unwrap_or(50))
}

/// 确认单条消息（仅手动 Ack / 预览模式）
#[tauri::command]
pub async fn ack_manual_consumer_message(
    consumer_id: String,
    message_id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    state
        .consumer_manager
        .ack_message(&consumer_id, &message_id)
        .await
}
