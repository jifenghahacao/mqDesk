//! 消息命令：发送 + 消息流 + 删除

use mqdesk_core::error::{AppError, AppResult};
use mqdesk_core::models::{
    FeedFilter, MessageDirection, MessageFeedItem, MessageStatus, PublishRequest, PublishResult,
};
use mqdesk_core::rabbit::AmqpPublisher;
use mqdesk_core::state::AppState;
use mqdesk_core::chrono::Utc;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn publish_message(
    request: PublishRequest,
    state: State<'_, Arc<AppState>>,
) -> AppResult<PublishResult> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }

    // JSON 校验
    if request.content_type.contains("json") {
        mqdesk_core::serde_json::from_str::<mqdesk_core::serde_json::Value>(&request.payload)
            .map_err(|_| AppError::InvalidJson)?;
    }

    // 解析目标队列（用于记录到 feed）
    let target_queue = request
        .target_queue
        .clone()
        .or_else(|| {
            // 经交换机模式：暂用 routing_key 作为标识
            if request.exchange.is_some() {
                Some(request.routing_key.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    // 提前保存 feed 需要的字段，因为 publish 会消费 request
    let payload_for_feed = request.payload.clone();
    let content_type_for_feed = request.content_type.clone();
    let exchange_for_feed = request.exchange.clone();
    let routing_key_for_feed = request.routing_key.clone();

    let amqp_url = state.amqp_url()?;
    let (vhost, exchange) = {
        let active = state.active_connection.read();
        let vhost = active
            .as_ref()
            .map(|a| a.connection.vhost.clone())
            .unwrap_or_default();
        let exchange = request.exchange.clone();
        (vhost, exchange)
    };

    let result = AmqpPublisher::publish(&amqp_url, &vhost, exchange.as_deref(), request).await?;

    // 写入消息流
    let time = Utc::now().to_rfc3339();
    let summary = format!("测试消息 · trace={}", &result.trace_id[..8.min(result.trace_id.len())]);
    let feed_status = match result.status {
        mqdesk_core::models::PublishStatus::Confirmed => MessageStatus::Sent,
        mqdesk_core::models::PublishStatus::Returned => MessageStatus::Failed,
        mqdesk_core::models::PublishStatus::Failed => MessageStatus::Failed,
    };

    let payload_preview = if payload_for_feed.len() > 500 {
        format!("{}…", &payload_for_feed[..500])
    } else {
        payload_for_feed.clone()
    };

    let feed_item = MessageFeedItem {
        trace_id: result.trace_id.clone(),
        time: time.clone(),
        direction: MessageDirection::Sent,
        queue_name: target_queue.clone(),
        exchange: exchange_for_feed,
        routing_key: routing_key_for_feed,
        status: feed_status,
        summary,
        payload_preview,
        payload_size: payload_for_feed.len() as u64,
        content_type: content_type_for_feed,
    };
    state.storage.append_feed(&feed_item)?;

    Ok(result)
}

#[tauri::command]
pub async fn list_message_feed(
    filter: Option<FeedFilter>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<MessageFeedItem>> {
    let filter = filter.unwrap_or(FeedFilter {
        queue: None,
        status: None,
        limit: Some(200),
    });
    let items = state.storage.list_feed(&filter)?;
    Ok(items)
}

#[tauri::command]
pub async fn get_message_trace(
    trace_id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<MessageFeedItem> {
    let item = state.storage.get_feed(&trace_id)?;
    Ok(item)
}

#[tauri::command]
pub async fn delete_message_trace(
    trace_id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()> {
    state.storage.delete_feed(&trace_id)?;
    Ok(())
}
