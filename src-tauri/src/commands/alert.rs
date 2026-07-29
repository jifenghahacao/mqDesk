//! 队列告警命令

use mqdesk_core::chrono::Utc;
use mqdesk_core::error::{AppError, AppResult};
use mqdesk_core::models::{Pagination, QueueAlertRecord, QueueAlertRule, QueueSummary};
use mqdesk_core::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn set_queue_alert_rule(
    rule: QueueAlertRule,
    state: State<'_, Arc<AppState>>,
) -> AppResult<QueueAlertRule> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    state.storage.set_alert_rule(&rule)?;
    Ok(rule)
}

#[tauri::command]
pub fn list_queue_alert_rules(
    queue_name: String,
    vhost: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<QueueAlertRule>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let rules = state.storage.list_alert_rules(Some(&queue_name), Some(&vhost))?;
    Ok(rules)
}

#[tauri::command]
pub fn delete_queue_alert_rule(
    queue_name: String,
    vhost: String,
    metric: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    state.storage.delete_alert_rule(&queue_name, &vhost, &metric)?;
    Ok(())
}

#[tauri::command]
pub fn list_queue_alert_records(
    queue_name: String,
    vhost: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<QueueAlertRecord>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let records = state.storage.list_alert_records(Some(&queue_name), Some(&vhost), 100)?;
    Ok(records)
}

/// 手动触发一次阈值检查。前端可周期性调用，返回本次新触发的告警。
#[tauri::command]
pub async fn check_queue_alerts(state: State<'_, Arc<AppState>>) -> AppResult<Vec<QueueAlertRecord>> {
    if state.get_active().is_none() {
        return Ok(Vec::new());
    }

    let rules = state.storage.list_alert_rules(None, None)?;
    if rules.is_empty() {
        return Ok(Vec::new());
    }

    let management = state.rabbit_management();
    let paginated = management
        .list_queues(&Default::default(), &Pagination::default())
        .await?;
    let summaries: Vec<QueueSummary> = paginated
        .items
        .into_iter()
        .map(|q| q.to_summary())
        .collect();

    let mut triggered = Vec::new();
    let now = Utc::now().to_rfc3339();

    for rule in rules.iter().filter(|r| r.enabled) {
        if let Some(summary) = summaries.iter().find(|s| s.name == rule.queue_name && s.vhost == rule.vhost) {
            let actual = match rule.metric.as_str() {
                "ready_count" => summary.ready as f64,
                "consumer_count" => summary.consumers as f64,
                "incoming_rate" => summary.incoming_rate,
                _ => continue,
            };

            let breached = match rule.operator.as_str() {
                "gt" => actual > rule.threshold,
                "eq" => (actual - rule.threshold).abs() < f64::EPSILON,
                "lt" => actual < rule.threshold,
                _ => false,
            };

            if breached {
                let record = QueueAlertRecord {
                    id: format!("{}-{}-{}", rule.queue_name, rule.metric, &now[..19]),
                    queue_name: rule.queue_name.clone(),
                    vhost: rule.vhost.clone(),
                    metric: rule.metric.clone(),
                    threshold: rule.threshold,
                    actual_value: actual,
                    triggered_at: now.clone(),
                    resolved_at: None,
                };
                state.storage.append_alert_record(&record)?;
                triggered.push(record);
            } else {
                state.storage.resolve_alert_record(&rule.queue_name, &rule.vhost, &rule.metric, &now)?;
            }
        }
    }

    Ok(triggered)
}
