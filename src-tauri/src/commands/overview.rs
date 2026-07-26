//! 总览命令

use mqdesk_core::error::{AppError, AppResult};
use mqdesk_core::models::{AlertItem, HealthStatus, Overview, OverviewStats, QueueFilter};
use mqdesk_core::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_overview(state: State<'_, Arc<AppState>>) -> AppResult<Overview> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let management = state.rabbit_management();

    let mgmt_overview = management.get_overview_stats().await?;
    let queues = management.list_queues(&QueueFilter::default()).await?;

    let mut queue_summaries: Vec<_> = queues.iter().map(|q| q.to_summary()).collect();
    queue_summaries.sort_by_key(|b| std::cmp::Reverse(b.ready));

    let alert_count = queue_summaries
        .iter()
        .filter(|q| matches!(q.health, HealthStatus::Warn | HealthStatus::Danger))
        .count() as u64;

    let total_messages = mgmt_overview
        .queue_totals
        .as_ref()
        .map(|t| t.messages)
        .unwrap_or(0);

    let overall_health = if queue_summaries.iter().any(|q| matches!(q.health, HealthStatus::Danger)) {
        HealthStatus::Danger
    } else if queue_summaries.iter().any(|q| matches!(q.health, HealthStatus::Warn)) {
        HealthStatus::Warn
    } else {
        HealthStatus::Ok
    };

    let (summary, summary_detail) = match overall_health {
        HealthStatus::Ok => (
            "✅ 一切正常".to_string(),
            "所有队列都在正常工作，无堆积、无未消费消息。".to_string(),
        ),
        HealthStatus::Warn => {
            let count = queue_summaries
                .iter()
                .filter(|q| matches!(q.health, HealthStatus::Warn))
                .count();
            (
                format!("⚠️ {} 个队列堆积预警", count),
                format!("有 {} 个队列待消费数偏高，建议检查消费端速率。", count),
            )
        }
        HealthStatus::Danger => {
            let count = queue_summaries
                .iter()
                .filter(|q| matches!(q.health, HealthStatus::Danger))
                .count();
            (
                format!("🔴 {} 个队列无人消费", count),
                format!(
                    "有 {} 个队列有消息待消费但无消费者在线，请确认消费程序是否运行。",
                    count
                ),
            )
        }
        HealthStatus::Idle => (
            "✅ 一切正常".to_string(),
            "所有队列都已处理完毕，暂无待处理消息。".to_string(),
        ),
    };

    let alerts: Vec<AlertItem> = queue_summaries
        .iter()
        .filter(|q| matches!(q.health, HealthStatus::Warn | HealthStatus::Danger))
        .map(|q| AlertItem {
            queue_name: q.name.clone(),
            health: q.health,
            ready: q.ready,
            reason: mqdesk_core::health::health_summary(&q.health),
        })
        .collect();

    let recent_feed = state
        .storage
        .list_feed(&mqdesk_core::models::FeedFilter {
            queue: None,
            status: None,
            limit: Some(5),
        })?
        .into_iter()
        .collect();

    Ok(Overview {
        health: overall_health,
        summary,
        summary_detail,
        stats: OverviewStats {
            queue_count: mgmt_overview.object_totals.queues,
            exchange_count: mgmt_overview.object_totals.exchanges,
            total_messages,
            alert_count,
        },
        alerts,
        recent_feed,
    })
}
