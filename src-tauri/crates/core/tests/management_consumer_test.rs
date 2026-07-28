//! RabbitMQ 消费者 API 测试（重点验证 N+1 查询修复）

use mqdesk_core::rabbit::ManagementClient;
use mockito::Matcher;

fn client_for_mock(server_url: &str) -> ManagementClient {
    let url = server_url.parse::<url::Url>().expect("mock URL 无效");
    let scheme = url.scheme();
    let host = url.host_str().unwrap_or("127.0.0.1");
    let port = url.port().unwrap_or(80);

    ManagementClient::new(scheme, host, port, "/", "guest", "guest")
}

#[tokio::test]
async fn test_list_consumers_uses_bulk_queue_fetch() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    // 50 个消费者，分布在 2 个队列上
    let consumers: Vec<_> = (0..50)
        .map(|i| {
            serde_json::json!({
                "consumer_tag": format!("consumer-{i}"),
                "channel_details": {
                    "name": format!("channel-{i}"),
                    "peer_host": "127.0.0.1",
                    "peer_port": 12345 + i,
                    "connection_name": format!("conn-{i}")
                },
                "queue": { "name": format!("queue-{}", i % 2), "vhost": "/" },
                "ack_required": true,
                "prefetch_count": 10
            })
        })
        .collect();

    let queues = serde_json::json!([
        {
            "name": "queue-0",
            "vhost": "/",
            "messages_ready": 100,
            "message_stats": { "deliver_get_details": { "rate": 10.5 } }
        },
        {
            "name": "queue-1",
            "vhost": "/",
            "messages_ready": 200,
            "message_stats": { "deliver_get_details": { "rate": 20.0 } }
        }
    ]);

    let connections = serde_json::json!([
        {
            "name": "conn-0",
            "connected_at": 1700000000000_u64
        }
    ]);

    let m_consumers = server
        .mock("GET", "/api/consumers/%2F")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(serde_json::to_string(&consumers).unwrap())
        .create();

    let m_queues = server
        .mock("GET", "/api/queues/%2F")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(queues.to_string())
        .create();

    let m_connections = server
        .mock("GET", "/api/connections")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(connections.to_string())
        .create();

    // 捕获所有针对单个队列的 GET 请求；若 N+1 未修复，这些请求会出现
    let m_single_queue = server
        .mock("GET", Matcher::Regex(r"^/api/queues/%2F/.+$".to_string()))
        .with_status(404)
        .expect(0)
        .create();

    let client = client_for_mock(&url);
    let result = client.list_consumers().await.expect("列出消费者应成功");

    assert_eq!(result.len(), 50);

    // 验证 message_rate 来自队列 bulk 拉取，而不是单个 get_queue
    let queue_0_rates: Vec<_> = result
        .iter()
        .filter(|c| c.queue_name == "queue-0")
        .map(|c| c.message_rate)
        .collect();
    let queue_1_rates: Vec<_> = result
        .iter()
        .filter(|c| c.queue_name == "queue-1")
        .map(|c| c.message_rate)
        .collect();
    assert!(queue_0_rates.iter().all(|&r| r == 10.5));
    assert!(queue_1_rates.iter().all(|&r| r == 20.0));

    m_consumers.assert();
    m_queues.assert();
    m_connections.assert();
    m_single_queue.assert();
}
