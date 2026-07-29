//! RabbitMQ 连接 API 测试

use mqdesk_core::models::Pagination;
use mqdesk_core::rabbit::ManagementClient;

fn client_for_mock(server_url: &str) -> ManagementClient {
    let url = server_url.parse::<url::Url>().expect("mock URL 无效");
    let scheme = url.scheme();
    let host = url.host_str().unwrap_or("127.0.0.1");
    let port = url.port().unwrap_or(80);

    ManagementClient::new(scheme, host, port, "/", "guest", "guest")
}

#[tokio::test]
async fn test_list_connections_success() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let body = serde_json::json!([
        {
            "name": "127.0.0.1:12345 -> 127.0.0.1:5672",
            "vhost": "/",
            "peer_host": "127.0.0.1",
            "peer_port": 12345,
            "protocol": "AMQP 0-9-1",
            "connected_at": 1700000000000_u64,
            "channels": 3,
            "state": "running"
        },
        {
            "name": "127.0.0.1:12346 -> 127.0.0.1:5672",
            "vhost": "/",
            "peer_host": "127.0.0.1",
            "peer_port": 12346,
            "protocol": "AMQP 0-9-1",
            "connected_at": 1700000001000_u64,
            "channels": 1,
            "state": "blocked"
        }
    ]);

    let m = server
        .mock("GET", "/api/connections?page=1&page_size=50")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(body.to_string())
        .create();

    let client = client_for_mock(&url);
    let result = client
        .list_connections(&Pagination { page: 1, page_size: 50 })
        .await
        .expect("列出连接应成功");

    assert_eq!(result.items.len(), 2);
    assert_eq!(result.items[0].peer_address, "127.0.0.1:12345");
    assert_eq!(result.items[0].channel_count, 3);
    assert_eq!(result.items[0].state, "running");
    assert_eq!(result.items[1].state, "blocked");
    assert_eq!(result.page, 1);
    assert_eq!(result.page_size, 50);

    m.assert();
}

#[tokio::test]
async fn test_list_connections_pagination_params() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let m = server
        .mock("GET", "/api/connections?page=2&page_size=10")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body("[]")
        .create();

    let client = client_for_mock(&url);
    let result = client
        .list_connections(&Pagination { page: 2, page_size: 10 })
        .await
        .expect("分页列出连接应成功");

    assert!(result.items.is_empty());
    assert_eq!(result.page, 2);
    assert_eq!(result.page_size, 10);

    m.assert();
}
