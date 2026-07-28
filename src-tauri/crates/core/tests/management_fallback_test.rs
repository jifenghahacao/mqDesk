//! ManagementClient 限流与降级缓存测试

use mqdesk_core::rabbit::ManagementClient;
use serde_json::json;

fn client_for_mock(server_url: &str) -> ManagementClient {
    // 从 mock server URL 解析 scheme/host/port
    let url = server_url.parse::<url::Url>().expect("mock URL 无效");
    let scheme = url.scheme();
    let host = url.host_str().unwrap_or("127.0.0.1");
    let port = url.port().unwrap_or(80);

    ManagementClient::new(scheme, host, port, "/", "guest", "guest")
}

#[tokio::test]
async fn test_overview_fallback_on_failure() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let body = json!({
        "object_totals": {
            "queues": 42,
            "exchanges": 10,
            "connections": 5,
            "consumers": 3
        },
        "queue_totals": {
            "messages": 100,
            "messages_ready": 80,
            "messages_unacknowledged": 20
        }
    });

    let m1 = server
        .mock("GET", "/api/overview")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body.to_string())
        .create();

    let m2 = server
        .mock("GET", "/api/overview")
        .with_status(500)
        .create();

    let client = client_for_mock(&url);

    // 第一次调用成功
    let overview = client.get_overview_stats().await.expect("首次调用应成功");
    assert_eq!(overview.object_totals.queues, 42);
    assert!(!client.is_stale());

    // 第二次调用失败，应返回缓存数据并标记 stale
    let overview2 = client.get_overview_stats().await.expect("降级应返回缓存");
    assert_eq!(overview2.object_totals.queues, 42);
    assert!(client.is_stale());

    m1.assert();
    m2.assert();
}

#[tokio::test]
async fn test_overview_failure_without_cache() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let m = server
        .mock("GET", "/api/overview")
        .with_status(500)
        .create();

    let client = client_for_mock(&url);

    // 首次即失败，没有缓存，应返回错误
    let result = client.get_overview_stats().await;
    assert!(result.is_err());
    assert!(!client.is_stale()); // 从未成功过，不标记 stale

    m.assert();
}
