//! 本地 RabbitMQ 端到端验证
//!
//! 前提：本地 RabbitMQ 运行在默认端口 5672/15672，账号 guest/guest。
//! 仅当设置环境变量 MQDESK_E2E=1 时执行，避免在 CI/日常测试中误操作真实 broker。
//!
//! 覆盖场景：
//! - 批量创建队列后分页列表加载耗时
//! - 发布消息 -> 队列详情 -> 清空 -> 验证消息数为 0
//! - 创建交换机/队列/绑定 -> 列表 -> 解绑
//! - 启动手动消费者后 Connections/Channels 非空

use base64::Engine;
use mqdesk_core::models::{CreateQueueInput, ManualConsumerConfig, Pagination, QueueFilter};
use mqdesk_core::rabbit::{ConsumerManager, ManagementClient};
use std::time::Instant;

fn e2e_enabled() -> bool {
    std::env::var("MQDESK_E2E").unwrap_or_default() == "1"
}

fn mgmt_client() -> ManagementClient {
    ManagementClient::new("http", "127.0.0.1", 15672, "/", "guest", "guest")
}

fn amqp_url() -> String {
    "amqp://guest:guest@127.0.0.1:5672".to_string()
}

fn unique_prefix() -> String {
    format!("mqdesk_e2e_{}_", std::process::id())
}

fn basic_auth() -> String {
    let credentials = "guest:guest";
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes())
    )
}

async fn fetch_json_array(url: &str, auth: &str) -> Vec<serde_json::Value> {
    let client = reqwest::Client::new();
    match client.get(url).header("Authorization", auth).send().await {
        Ok(resp) => resp.json().await.unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

async fn cleanup_by_prefix(prefix: &str) {
    let client = reqwest::Client::new();
    let auth = basic_auth();

    // 清理队列
    let queues = fetch_json_array("http://127.0.0.1:15672/api/queues/%2f", &auth).await;
    for q in queues {
        if let Some(name) = q.get("name").and_then(|n| n.as_str()) {
            if name.starts_with(prefix) {
                let _ = client
                    .delete(format!("http://127.0.0.1:15672/api/queues/%2f/{}", name))
                    .header("Authorization", &auth)
                    .send()
                    .await;
            }
        }
    }

    // 清理交换机
    let exchanges = fetch_json_array("http://127.0.0.1:15672/api/exchanges/%2f", &auth).await;
    for e in exchanges {
        if let Some(name) = e.get("name").and_then(|n| n.as_str()) {
            if name.starts_with(prefix) && !name.is_empty() {
                let _ = client
                    .delete(format!("http://127.0.0.1:15672/api/exchanges/%2f/{}", name))
                    .header("Authorization", &auth)
                    .send()
                    .await;
            }
        }
    }
}

async fn create_exchange_http(name: &str) {
    let client = reqwest::Client::new();
    let body = serde_json::json!({"type":"direct","durable":false,"auto_delete":true});
    let _ = client
        .put(format!("http://127.0.0.1:15672/api/exchanges/%2f/{}", name))
        .header("Authorization", basic_auth())
        .json(&body)
        .send()
        .await;
}

async fn create_binding_http(exchange: &str, queue: &str, routing_key: &str) {
    let client = reqwest::Client::new();
    let body = serde_json::json!({"routing_key": routing_key});
    let _ = client
        .post(format!(
            "http://127.0.0.1:15672/api/bindings/%2f/e/{}/q/{}",
            exchange, queue
        ))
        .header("Authorization", basic_auth())
        .json(&body)
        .send()
        .await;
}

async fn publish_messages_to_exchange(exchange: &str, routing_key: &str, count: usize) {
    let client = reqwest::Client::new();
    let auth = basic_auth();
    for i in 0..count {
        let body = serde_json::json!({
            "routing_key": routing_key,
            "payload": format!("{{\"n\":{}}}", i),
            "payload_encoding": "string",
            "properties": {}
        });
        let _ = client
            .post(format!(
                "http://127.0.0.1:15672/api/exchanges/%2f/{}/publish",
                exchange
            ))
            .header("Authorization", &auth)
            .json(&body)
            .send()
            .await;
    }
}

#[tokio::test]
async fn e2e_rabbitmq_live() {
    if !e2e_enabled() {
        return;
    }

    let client = mgmt_client();
    let prefix = unique_prefix();
    let name = |suffix: &str| format!("{}{}", prefix, suffix);

    // 0. 总览可访问
    let overview = client.get_overview_stats().await.expect("应能获取 overview");
    assert!(overview.object_totals.queues <= 100000);

    // 1. 批量创建队列（100 个，用于验证分页）
    const BULK_COUNT: usize = 100;
    let start = Instant::now();
    for i in 0..BULK_COUNT {
        let input = CreateQueueInput {
            name: name(&format!("bulk_{:03}", i)),
            vhost: "/".to_string(),
            queue_type: "classic".to_string(),
            durable: true,
            auto_delete: true,
            arguments: serde_json::Value::Object(Default::default()),
        };
        client.create_queue(&input).await.expect("创建队列应成功");
    }
    let create_elapsed = start.elapsed();
    println!("创建 {} 个队列耗时: {:?}", BULK_COUNT, create_elapsed);

    // 2. 分页列表：首屏 50 条应 ≤ 1.5s
    let start = Instant::now();
    let paginated = client
        .list_queues(&QueueFilter::default(), &Pagination { page: 1, page_size: 50 })
        .await
        .expect("分页列表应成功");
    let list_elapsed = start.elapsed();
    println!(
        "分页列表首屏耗时: {:?}, 本页 {} 条, total {}",
        list_elapsed,
        paginated.items.len(),
        paginated.total
    );
    assert!(
        list_elapsed.as_secs_f64() <= 1.5,
        "首屏加载应 ≤ 1.5s, 实际 {:?}",
        list_elapsed
    );
    assert_eq!(paginated.items.len(), 50);

    // 3. 发布 1000 条消息到目标队列，然后清空
    let target_queue = name("purge_target");
    client
        .create_queue(&CreateQueueInput {
            name: target_queue.clone(),
            vhost: "/".to_string(),
            queue_type: "classic".to_string(),
            durable: true,
            auto_delete: false,
            arguments: serde_json::Value::Object(Default::default()),
        })
        .await
        .expect("创建目标队列应成功");

    let target_exchange = name("purge_ex");
    create_exchange_http(&target_exchange).await;
    create_binding_http(&target_exchange, &target_queue, "purge.route").await;
    publish_messages_to_exchange(&target_exchange, "purge.route", 1000).await;

    // 等待 Management API 统计刷新（RabbitMQ 统计有秒级延迟，非 auto_delete 队列可能更慢）
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let detail = client
        .get_queue(&target_queue)
        .await
        .expect("获取队列详情应成功");
    println!(
        "发布 1000 条后队列详情: name={}, messages={}, ready={}",
        target_queue, detail.messages, detail.messages_ready
    );
    assert!(detail.messages >= 1000, "队列应包含至少 1000 条消息, 实际 {}", detail.messages);

    client.purge_queue(&target_queue).await.expect("清空队列应成功");

    // purge 后 Management API 统计更新亦有延迟，轮询最多 15 秒等待归零
    let mut purged_detail = None;
    for _ in 0..30 {
        match client.get_queue(&target_queue).await {
            Ok(d) if d.messages == 0 => {
                purged_detail = Some(d);
                break;
            }
            Ok(d) => purged_detail = Some(d),
            Err(e) => println!("清空后查询队列详情失败（将重试）: {}", e),
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    let detail = purged_detail.expect("清空后应能获取队列详情");
    assert_eq!(detail.messages, 0, "清空后消息数应为 0");

    // 4. 绑定：通过 HTTP 创建交换机/绑定，再用 ManagementClient 列表并解绑
    let exchange = name("bind_ex");
    let bind_queue = name("bind_q");
    let routing_key = "e2e.route";

    client
        .create_queue(&CreateQueueInput {
            name: bind_queue.clone(),
            vhost: "/".to_string(),
            queue_type: "classic".to_string(),
            durable: true,
            auto_delete: true,
            arguments: serde_json::Value::Object(Default::default()),
        })
        .await
        .expect("创建绑定队列应成功");

    create_exchange_http(&exchange).await;
    create_binding_http(&exchange, &bind_queue, routing_key).await;

    let bindings = client
        .list_queue_bindings(&bind_queue)
        .await
        .expect("列出绑定应成功");
    let found = bindings
        .iter()
        .any(|b| b.source == exchange && b.routing_key == routing_key);
    assert!(found, "应能查找到新建绑定");

    let binding = bindings
        .into_iter()
        .find(|b| b.source == exchange && b.routing_key == routing_key)
        .expect("绑定应存在");
    client
        .delete_binding(&binding)
        .await
        .expect("删除绑定应成功");

    // 5. 消费者：启动手动消费者，验证 Connections/Channels 非空
    let consumer_queue = name("consumer_q");
    client
        .create_queue(&CreateQueueInput {
            name: consumer_queue.clone(),
            vhost: "/".to_string(),
            queue_type: "classic".to_string(),
            durable: true,
            auto_delete: true,
            arguments: serde_json::Value::Object(Default::default()),
        })
        .await
        .expect("创建消费者队列应成功");

    let consumer_manager = ConsumerManager::new();
    let consumer = consumer_manager
        .create(ManualConsumerConfig {
            name: name("consumer"),
            queue_name: consumer_queue.clone(),
            mode: "async".to_string(),
            prefetch_count: 10,
            auto_ack: true,
            filter: Default::default(),
        })
        .expect("创建消费者应成功");

    let consumer_after_start = consumer_manager
        .start(&consumer.id, &amqp_url())
        .await
        .expect("启动消费者应成功");
    println!(
        "消费者启动后状态: id={}, status={}, error={:?}",
        consumer_after_start.id, consumer_after_start.status, consumer_after_start.error
    );

    // 轮询等待 Management API 统计更新：连接、信道、队列消费者
    let mut connections = None;
    let mut channels = None;
    let mut consumer_q_detail = None;
    for i in 0..30 {
        if connections.is_none() {
            if let Ok(c) = client.list_connections(&Pagination::default()).await {
                if !c.items.is_empty() {
                    connections = Some(c);
                }
            }
        }
        if channels.is_none() {
            if let Ok(c) = client.list_channels(None, &Pagination::default()).await {
                if !c.items.is_empty() {
                    channels = Some(c);
                }
            }
        }
        if consumer_q_detail.is_none() {
            if let Ok(d) = client.get_queue(&consumer_queue).await {
                consumer_q_detail = Some(d);
            }
        }
        if connections.is_some() && channels.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if i == 29 {
            println!("警告：轮询 15 秒后仍未在 Management API 中看到连接/信道");
        }
    }

    let consumer_q_detail = consumer_q_detail.expect("应能获取消费者队列详情");
    println!(
        "消费者队列状态: consumers={}, messages={}",
        consumer_q_detail.consumers, consumer_q_detail.messages
    );

    let _connections = connections.expect("应至少存在一个连接");
    let _channels = channels.expect("应至少存在一个信道");

    consumer_manager
        .destroy(&consumer.id)
        .await
        .expect("销毁消费者应成功");

    // 显式清理测试资源
    cleanup_by_prefix(&prefix).await;

    println!("E2E 验证全部通过");
}
