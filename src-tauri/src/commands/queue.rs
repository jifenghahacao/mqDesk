//! 队列命令

use crate::commands::audit;
use mqdesk_core::error::{AppError, AppResult};
use mqdesk_core::models::{
    CreateQueueInput, PreviewMessage, QueueDetail, QueueFilter, QueueMessage, QueuePolicyInput,
    QueueSummary, RateHistory,
};
use mqdesk_core::state::AppState;
use std::sync::Arc;
use tauri::State;

fn active_vhost(state: &AppState) -> String {
    state.get_active().map(|c| c.vhost).unwrap_or_default()
}

#[tauri::command]
pub async fn list_queues(
    filter: QueueFilter,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<QueueSummary>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let management = state.rabbit_management();
    let queues = management.list_queues(&filter).await?;
    let mut summaries: Vec<_> = queues.iter().map(|q| q.to_summary()).collect();

    if !filter.health.is_empty() && filter.health != "all" {
        summaries.retain(|q| q.health.css_class() == filter.health);
    }

    summaries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(summaries)
}

#[tauri::command]
pub async fn get_queue_detail(
    name: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<QueueDetail> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let management = state.rabbit_management();
    let queue = management.get_queue(&name).await?;

    let history = generate_rate_history(
        queue
            .message_stats
            .as_ref()
            .and_then(|s| s.publish_details.as_ref())
            .map(|d| d.rate)
            .unwrap_or(0.0),
        queue
            .message_stats
            .as_ref()
            .and_then(|s| s.deliver_get_details.as_ref())
            .map(|d| d.rate)
            .unwrap_or(0.0),
    );

    Ok(queue.to_detail(history))
}

#[tauri::command]
pub async fn grab_preview(
    queue_name: String,
    count: Option<u32>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<PreviewMessage>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let n = count.unwrap_or(10).clamp(1, 100);
    let management = state.rabbit_management();
    let messages = management.get_messages_preview(&queue_name, n).await?;
    Ok(messages)
}

#[tauri::command]
pub async fn peek_queue_messages(
    queue_name: String,
    count: Option<u32>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<QueueMessage>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let n = count.unwrap_or(20).clamp(1, 100);
    let management = state.rabbit_management();
    let messages = management.peek_queue_messages(&queue_name, n).await?;
    Ok(messages)
}

#[tauri::command]
pub async fn create_queue(
    config: CreateQueueInput,
    state: State<'_, Arc<AppState>>,
) -> AppResult<QueueSummary> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let management = state.rabbit_management();
    management.create_queue(&config).await?;
    let queue = management.get_queue(&config.name).await?;
    let _ = audit::record(
        &**state,
        "create_queue",
        &config.name,
        &config.vhost,
        &format!("type={}, durable={}", config.queue_type, config.durable),
    );
    Ok(queue.to_summary())
}

#[tauri::command]
pub async fn update_queue_policy(
    input: QueuePolicyInput,
    state: State<'_, Arc<AppState>>,
) -> AppResult<QueueSummary> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let management = state.rabbit_management();
    management.update_queue_policy(&input).await?;
    let queue = management.get_queue(&input.name).await?;
    let detail = format!(
        "max_length={:?}, ttl={:?}, dlx={:?}, dlrk={:?}",
        input.max_length, input.message_ttl, input.dead_letter_exchange, input.dead_letter_routing_key
    );
    let _ = audit::record(&**state, "update_queue_policy", &input.name, &input.vhost, &detail);
    Ok(queue.to_summary())
}

#[tauri::command]
pub async fn delete_queue(
    name: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let vhost = active_vhost(&state);
    let management = state.rabbit_management();
    management.delete_queue(&name).await?;
    let _ = audit::record(&**state, "delete_queue", &name, &vhost, "队列已删除");
    Ok(())
}

#[tauri::command]
pub async fn pause_queue(
    name: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<QueueSummary> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let vhost = active_vhost(&state);
    let management = state.rabbit_management();
    management.pause_queue(&name).await?;
    let queue = management.get_queue(&name).await?;
    let _ = audit::record(&**state, "pause_queue", &name, &vhost, "队列已暂停");
    Ok(queue.to_summary())
}

#[tauri::command]
pub async fn resume_queue(
    name: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<QueueSummary> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let vhost = active_vhost(&state);
    let management = state.rabbit_management();
    management.resume_queue(&name).await?;
    let queue = management.get_queue(&name).await?;
    let _ = audit::record(&**state, "resume_queue", &name, &vhost, "队列已恢复");
    Ok(queue.to_summary())
}

/// 生成 12 点历史速率数据
/// 注意：v1 简化版用当前速率抖动生成，未来应替换为 sled 中的实际历史采样
fn generate_rate_history(incoming: f64, outgoing: f64) -> RateHistory {
    use mqdesk_core::chrono::Utc;
    let mut next = LinearCongruential::new();
    let n = 12;
    let mut in_pts = Vec::with_capacity(n);
    let mut out_pts = Vec::with_capacity(n);
    let mut base_in = incoming.max(0.0);
    let mut base_out = outgoing.max(0.0);

    let now = Utc::now();
    let timestamps: Vec<String> = (0..n)
        .map(|i| {
            let t = now - mqdesk_core::chrono::Duration::minutes((n - 1 - i) as i64 * 5);
            t.format("%H:%M").to_string()
        })
        .collect();

    for _ in 0..n {
        base_in = (base_in + next.next_f64() * 6.0 - 3.0).max(0.0);
        base_out = (base_out + next.next_f64() * 6.0 - 3.0).max(0.0);
        in_pts.push(base_in);
        out_pts.push(base_out);
    }
    RateHistory {
        incoming: in_pts,
        outgoing: out_pts,
        timestamps,
    }
}

struct LinearCongruential {
    state: u64,
}

impl LinearCongruential {
    fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(42);
        Self { state: seed }
    }

    fn next_f64(&mut self) -> f64 {
        // LCG 参数：numerical recipes
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.state as f64) / (u64::MAX as f64)
    }
}
