//! 真实 RabbitMQ 冒烟测试
//!
//! 前置条件：本地运行 RabbitMQ（含 management 插件），端口 5672/15672，guest/guest 可访问。
//! 覆盖 PRD R1-R7 的核心后端逻辑：
//! - R1 连接测试（ManagementClient::test_connection）
//! - R2 总览（ManagementClient::get_overview_stats）
//! - R3 队列列表（ManagementClient::list_queues）
//! - R4 队列详情 + 抓取预览 requeue（ManagementClient::get_queue + get_messages_preview）
//! - R5 引导式发送（AmqpPublisher::publish，publisher confirms + mandatory）
//! - R6 健康度四态判定（health::judge_health 用真实队列数据）
//! - R7 消息流存储（Storage::append_feed + list_feed）
//!
//! 所有测试资源用 "mqdesk-smoke-" 前缀，测试结束清理。

use mqdesk_core::health::judge_health;
use mqdesk_core::models::{
    FeedFilter, HealthStatus, MessageDirection, MessageFeedItem, MessageStatus, Pagination, PublishRequest,
    PublishStatus, QueueFilter,
};
use mqdesk_core::rabbit::{AmqpPublisher, ManagementClient};
use mqdesk_core::storage::Storage;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn unique(suffix: &str) -> String {
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("mqdesk-smoke-{suffix}-{ts}-{n}")
}

/// 直接调 Management API 做 setup/cleanup（mqdesk-core 暂不封装写操作）
struct MgmtHelper {
    client: reqwest::Client,
    base: String,
    auth: String,
}

impl MgmtHelper {
    fn new() -> Self {
        use base64::Engine;
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
            base: "http://127.0.0.1:15672".to_string(),
            auth: format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(b"guest:guest")
            ),
        }
    }

    async fn put(&self, path: &str, body: serde_json::Value) {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .client
            .put(&url)
            .header("Authorization", &self.auth)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .unwrap_or_else(|e| panic!("PUT {path} 失败：{e}"));
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            panic!("PUT {path} 返回 {status}，响应体：{text}");
        }
    }

    async fn delete(&self, path: &str) {
        let url = format!("{}{}", self.base, path);
        let _ = self
            .client
            .delete(&url)
            .header("Authorization", &self.auth)
            .send()
            .await;
    }

    async fn create_queue(&self, vhost: &str, name: &str) {
        let path = format!("/api/queues/{}/{}", enc(vhost), enc(name));
        // RabbitMQ 4.x 默认禁用 transient_nonexcl_queues，必须用 durable=true
        self.put(
            &path,
            serde_json::json!({"durable": true, "auto_delete": false, "arguments": {}}),
        )
        .await;
    }

    async fn delete_queue(&self, vhost: &str, name: &str) {
        let path = format!("/api/queues/{}/{}", enc(vhost), enc(name));
        self.delete(&path).await;
    }

    async fn purge_queue(&self, vhost: &str, name: &str) {
        let url = format!("{}/api/queues/{}/{}/contents", self.base, enc(vhost), enc(name));
        let _ = self
            .client
            .delete(&url)
            .header("Authorization", &self.auth)
            .send()
            .await;
    }
}

fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn mgmt() -> ManagementClient {
    ManagementClient::new("http", "127.0.0.1", 15672, "/", "guest", "guest")
}

fn amqp_url() -> String {
    // vhost "/" 必须编码为 %2F，否则 AMQP 会当成空 vhost 报 "vhost not found"
    "amqp://guest:guest@127.0.0.1:5672/%2F".to_string()
}

#[tokio::test]
async fn r1_test_connection_whoami() {
    let name = mgmt().test_connection().await.expect("whoami 应成功");
    assert_eq!(name, "guest", "whoami 应返回 guest");
}

#[tokio::test]
async fn r2_overview_stats() {
    let overview = mgmt()
        .get_overview_stats()
        .await
        .expect("总览应成功");
    assert!(overview.object_totals.queues < 10000, "队列数应在合理范围");
    assert!(overview.object_totals.exchanges >= 7, "至少有默认交换机");
}

#[tokio::test]
async fn r3_r4_r6_queue_list_detail_preview_health() {
    let helper = MgmtHelper::new();
    let qname = unique("q");
    let vhost = "/";

    // setup：建队
    helper.create_queue(vhost, &qname).await;

    // R3：列表里能找到
    let paginated = mgmt()
        .list_queues(&QueueFilter::default(), &Pagination::default())
        .await
        .expect("队列列表应成功");
    let found = paginated.items.iter().any(|q| q.name == qname);
    assert!(found, "新建队列 {qname} 应出现在列表中");

    // R6：空队列 → Idle
    let detail = mgmt().get_queue(&qname).await.expect("队列详情应成功");
    let health = judge_health(
        detail.messages_ready,
        detail.consumers,
        0.0,
        0.0,
    );
    assert_eq!(health, HealthStatus::Idle, "空队列应为 Idle");

    // 往队列塞一条消息（用 default exchange 直发）
    let req = PublishRequest {
        target_queue: Some(qname.clone()),
        exchange: None,
        routing_key: qname.clone(),
        payload: r#"{"smoke":"r3r4r6"}"#.to_string(),
        content_type: "application/json".to_string(),
        headers: serde_json::json!({}),
        delivery_mode: 2,
        mandatory: true,
    };
    let result = AmqpPublisher::publish(&amqp_url(), vhost, None, req)
        .await
        .expect("发布应成功");
    assert_eq!(
        result.status,
        PublishStatus::Confirmed,
        "应有 publisher confirm，错误：{}",
        result.error
    );

    // RabbitMQ 4.x stats 异步收集，需等刷新周期（~5s）才能看到 messages_ready
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;

    // R6：有消息无消费者 → Danger
    let detail = mgmt().get_queue(&qname).await.expect("队列详情应成功");
    assert_eq!(detail.messages_ready, 1, "应有 1 条 ready 消息");
    let health = judge_health(
        detail.messages_ready,
        detail.consumers,
        0.0,
        0.0,
    );
    assert_eq!(health, HealthStatus::Danger, "有消息无消费者应为 Danger");

    // R4：抓取预览（requeue=true）
    let preview = mgmt()
        .get_messages_preview(&qname, 5)
        .await
        .expect("抓取预览应成功");
    assert_eq!(preview.len(), 1, "应抓到 1 条消息");
    assert!(preview[0].payload_preview.contains("smoke"), "payload 应包含 smoke");

    // 关键：requeue 后消息还在（等 stats 刷新）
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;
    let after = mgmt().get_queue(&qname).await.expect("再次取详情应成功");
    assert_eq!(
        after.messages_ready, 1,
        "requeue=true 抓取后消息应仍在队列中"
    );

    // cleanup
    helper.delete_queue(vhost, &qname).await;
}

#[tokio::test]
async fn r5_publish_mandatory_returned() {
    let helper = MgmtHelper::new();
    let vhost = "/";
    // 用一个不存在的队列做 routing_key，mandatory=true 应触发 basic.return
    let ghost = unique("ghost");

    let req = PublishRequest {
        target_queue: None,
        exchange: None, // default exchange
        routing_key: ghost.clone(),
        payload: r#"{"smoke":"mandatory"}"#.to_string(),
        content_type: "application/json".to_string(),
        headers: serde_json::json!({}),
        delivery_mode: 2,
        mandatory: true,
    };
    let result = AmqpPublisher::publish(&amqp_url(), vhost, None, req)
        .await
        .expect("发布调用应不报错");
    assert_eq!(
        result.status,
        PublishStatus::Returned,
        "mandatory + 不存在队列应 Returned，实际：{:?} (err={})",
        result.status,
        result.error
    );
    assert!(result.reply_code != 0, "Returned 应带 reply_code");
}

#[tokio::test]
async fn r5_publish_invalid_json_rejected_by_caller() {
    // 调用方（commands::message）会校验 JSON，这里只验证 publisher 本身能处理非 JSON payload
    let helper = MgmtHelper::new();
    let qname = unique("rawtext");
    helper.create_queue("/", &qname).await;

    let req = PublishRequest {
        target_queue: Some(qname.clone()),
        exchange: None,
        routing_key: qname.clone(),
        payload: "plain text 不是 json".to_string(),
        content_type: "text/plain".to_string(),
        headers: serde_json::json!({}),
        delivery_mode: 2,
        mandatory: true,
    };
    let result = AmqpPublisher::publish(&amqp_url(), "/", None, req)
        .await
        .expect("发布应成功");
    assert_eq!(result.status, PublishStatus::Confirmed);

    helper.delete_queue("/", &qname).await;
}

#[tokio::test]
async fn r7_storage_feed_append_list_filter_delete() {
    let tmp = tempfile::tempdir().expect("创建临时目录失败");
    let storage = Storage::open(tmp.path().to_path_buf()).expect("打开 sled 失败");

    let item = MessageFeedItem {
        trace_id: unique("trace"),
        time: chrono::Utc::now().to_rfc3339(),
        direction: MessageDirection::Sent,
        queue_name: "mqdesk-smoke-feed-q".to_string(),
        exchange: None,
        routing_key: "mqdesk.smoke".to_string(),
        status: MessageStatus::Sent,
        summary: "冒烟测试消息".to_string(),
        payload_preview: r#"{"test":true}"#.to_string(),
        payload_size: 15,
        content_type: "application/json".to_string(),
    };
    storage.append_feed(&item).expect("写入 feed 失败");

    // list 全量
    let all = storage
        .list_feed(&FeedFilter {
            queue: None,
            status: None,
            limit: Some(50),
        })
        .expect("查询 feed 失败");
    assert_eq!(all.len(), 1, "应有 1 条记录");
    assert_eq!(all[0].trace_id, item.trace_id);

    // 按队列过滤
    let filtered = storage
        .list_feed(&FeedFilter {
            queue: Some("mqdesk-smoke-feed-q".to_string()),
            status: None,
            limit: Some(50),
        })
        .expect("过滤查询失败");
    assert_eq!(filtered.len(), 1, "队列过滤应有 1 条");

    // 状态更新（Sent → Backlog）
    storage
        .update_feed_status(&item.trace_id, MessageStatus::Backlog)
        .expect("更新状态失败");
    let after = storage
        .list_feed(&FeedFilter {
            queue: None,
            status: None,
            limit: Some(50),
        })
        .expect("查询失败");
    assert_eq!(after[0].status, MessageStatus::Backlog, "状态应为 Backlog");

    // 删除
    storage
        .delete_feed(&item.trace_id)
        .expect("删除 feed 失败");
    let empty = storage
        .list_feed(&FeedFilter {
            queue: None,
            status: None,
            limit: Some(50),
        })
        .expect("查询失败");
    assert!(empty.is_empty(), "删除后应为空");
}
