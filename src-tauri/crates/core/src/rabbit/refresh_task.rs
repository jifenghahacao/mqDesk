//! 后台自动刷新任务：周期性拉取队列状态并通过事件发射器推送

use crate::models::{HealthStatus, Pagination, QueueFilter, QueueRefreshEvent, QueueSummary};
use crate::state::AppState;
use parking_lot::{Mutex, RwLock};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::time::interval;

/// 刷新事件发射器：由 Tauri 壳实现，将事件投递到前端。
pub trait RefreshEventEmitter: Send + Sync {
    fn emit_queue_refreshed(&self, event: QueueRefreshEvent);
    fn emit_management_stale(&self, is_stale: bool);
}

/// 自动刷新任务，绑定到单个活跃 RabbitMQ 连接。
pub struct RefreshTask {
    emitter: Arc<RwLock<Option<Arc<dyn RefreshEventEmitter>>>>,
    interval_ms: Arc<AtomicU64>,
    enabled: Arc<AtomicBool>,
    running: AtomicBool,
    handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Default for RefreshTask {
    fn default() -> Self {
        Self::new()
    }
}

impl RefreshTask {
    pub fn new() -> Self {
        Self {
            emitter: Arc::new(RwLock::new(None)),
            interval_ms: Arc::new(AtomicU64::new(5000)),
            enabled: Arc::new(AtomicBool::new(true)),
            running: AtomicBool::new(false),
            handle: Mutex::new(None),
        }
    }

    /// 设置事件发射器，通常由 Tauri 壳在启动时注入。
    pub fn set_emitter(&self, emitter: Arc<dyn RefreshEventEmitter>) {
        *self.emitter.write() = Some(emitter);
    }

    /// 设置刷新周期（毫秒），最小生效值为 1000ms。
    pub fn set_interval(&self, ms: u64) {
        self.interval_ms.store(ms, Ordering::SeqCst);
    }

    /// 启用或禁用自动刷新。
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 启动后台刷新循环。已启动时直接返回。
    pub fn start(&self, state: Weak<AppState>) {
        let mut handle_guard = self.handle.lock();
        if handle_guard.is_some() {
            return;
        }
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let interval_ms = self.interval_ms.clone();
        let enabled = self.enabled.clone();
        let emitter = self.emitter.clone();

        let handle = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(
                interval_ms.load(Ordering::SeqCst).max(1000),
            ));
            loop {
                ticker.tick().await;
                if !enabled.load(Ordering::SeqCst) {
                    continue;
                }
                let Some(state) = state.upgrade() else {
                    break;
                };
                refresh_once(&state, &emitter).await;
            }
        });

        *handle_guard = Some(handle);
    }

    /// 停止后台刷新循环。
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        let handle = self.handle.lock().take();
        if let Some(h) = handle {
            h.abort();
        }
    }
}

async fn refresh_once(
    state: &AppState,
    emitter: &Arc<RwLock<Option<Arc<dyn RefreshEventEmitter>>>>,
) {
    if state.get_active().is_none() {
        return;
    }
    let management = state.rabbit_management();

    let filter = QueueFilter::default();
    let pagination = Pagination {
        page: 1,
        page_size: 500,
    };

    match management.list_queues(&filter, &pagination).await {
        Ok(paginated) => {
            let queues: Vec<QueueSummary> =
                paginated.items.iter().map(|q| q.to_summary()).collect();
            let is_stale = management.is_stale();
            let overall_health = compute_overall_health(&queues);
            let alert_count = queues
                .iter()
                .filter(|q| matches!(q.health, HealthStatus::Warn | HealthStatus::Danger))
                .count() as u64;

            let event = QueueRefreshEvent {
                queues,
                overall_health,
                alert_count,
                is_stale,
            };

            if let Some(emitter) = emitter.read().as_ref() {
                emitter.emit_management_stale(is_stale);
                emitter.emit_queue_refreshed(event);
            }
        }
        Err(e) => {
            log::warn!("RefreshTask 拉取队列失败：{}", e);
            if let Some(emitter) = emitter.read().as_ref() {
                emitter.emit_management_stale(true);
            }
        }
    }
}

fn compute_overall_health(queues: &[QueueSummary]) -> HealthStatus {
    if queues
        .iter()
        .any(|q| matches!(q.health, HealthStatus::Danger))
    {
        HealthStatus::Danger
    } else if queues
        .iter()
        .any(|q| matches!(q.health, HealthStatus::Warn))
    {
        HealthStatus::Warn
    } else {
        HealthStatus::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Connection;
    use crate::storage::Storage;
    use std::sync::atomic::AtomicUsize;

    #[derive(Default)]
    struct MockEmitter {
        refreshed: AtomicUsize,
        stale_calls: AtomicUsize,
        last_stale: AtomicBool,
    }

    impl MockEmitter {
        fn refreshed_count(&self) -> usize {
            self.refreshed.load(Ordering::SeqCst)
        }
        fn stale_count(&self) -> usize {
            self.stale_calls.load(Ordering::SeqCst)
        }
        fn last_stale(&self) -> bool {
            self.last_stale.load(Ordering::SeqCst)
        }
    }

    impl RefreshEventEmitter for MockEmitter {
        fn emit_queue_refreshed(&self, _event: QueueRefreshEvent) {
            self.refreshed.fetch_add(1, Ordering::SeqCst);
        }

        fn emit_management_stale(&self, is_stale: bool) {
            self.stale_calls.fetch_add(1, Ordering::SeqCst);
            self.last_stale.store(is_stale, Ordering::SeqCst);
        }
    }

    fn tmp_storage() -> Storage {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        Storage::open(dir.path().to_path_buf()).expect("打开 storage 失败")
    }

    fn extract_port(url: &str) -> u16 {
        url.split(':')
            .last()
            .and_then(|s| s.parse().ok())
            .unwrap_or(15672)
    }

    fn test_connection(management_port: u16) -> Connection {
        Connection {
            id: "refresh-test".to_string(),
            name: "refresh-test".to_string(),
            host: "127.0.0.1".to_string(),
            amqp_port: 5672,
            management_port,
            management_scheme: "http".to_string(),
            vhost: "/".to_string(),
            username: "guest".to_string(),
            password: "guest".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn refresh_task_triggers_periodically() {
        let mut server = mockito::Server::new_async().await;
        let body = serde_json::json!([{
            "name": "q1",
            "vhost": "/",
            "type": "classic",
            "durable": true,
            "auto_delete": false,
            "arguments": {},
            "messages": 0,
            "messages_ready": 0,
            "messages_unacknowledged": 0,
            "consumers": 0,
        }]);
        let _m = server
            .mock("GET", "/api/queues/%2F?page=1&page_size=500")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create();

        let state = Arc::new(AppState::with_storage(tmp_storage()));
        let emitter = Arc::new(MockEmitter::default());
        state.refresh_task.set_emitter(emitter.clone());
        state.refresh_task.set_interval(100);
        state.refresh_task.set_enabled(true);
        state.set_active(test_connection(extract_port(&server.url())), "guest".to_string());

        tokio::time::sleep(Duration::from_millis(250)).await;
        state.refresh_task.stop();

        assert!(
            emitter.refreshed_count() >= 1,
            "应在 250ms 内至少触发 1 次刷新"
        );
        assert_eq!(emitter.last_stale(), false, "mock 返回成功时不应 stale");
    }

    #[tokio::test]
    async fn refresh_task_respects_enabled_flag() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/queues/%2F?page=1&page_size=500")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[]")
            .create();

        let state = Arc::new(AppState::with_storage(tmp_storage()));
        let emitter = Arc::new(MockEmitter::default());
        state.refresh_task.set_emitter(emitter.clone());
        state.refresh_task.set_interval(100);
        state.refresh_task.set_enabled(false);
        state.set_active(test_connection(extract_port(&server.url())), "guest".to_string());

        tokio::time::sleep(Duration::from_millis(250)).await;
        state.refresh_task.stop();

        assert_eq!(
            emitter.refreshed_count(),
            0,
            "禁用时不应触发刷新事件"
        );
    }

    #[tokio::test]
    async fn refresh_task_emits_stale_on_failure() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/queues/%2F?page=1&page_size=500")
            .with_status(503)
            .create();

        let state = Arc::new(AppState::with_storage(tmp_storage()));
        let emitter = Arc::new(MockEmitter::default());
        state.refresh_task.set_emitter(emitter.clone());
        state.refresh_task.set_interval(100);
        state.refresh_task.set_enabled(true);
        state.set_active(test_connection(extract_port(&server.url())), "guest".to_string());

        tokio::time::sleep(Duration::from_millis(250)).await;
        state.refresh_task.stop();

        assert!(
            emitter.stale_count() >= 1,
            "失败时应至少触发 1 次 stale 事件"
        );
        assert!(emitter.last_stale(), "失败时 stale 应为 true");
    }
}
