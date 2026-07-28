//! RabbitMQ 信道 API 测试

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
async fn test_list_channels_success_and_filter() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let body = serde_json::json!([
        {
            "name": "127.0.0.1:12345 -> 127.0.0.1:5672 (1)",
            "vhost": "/",
            "connection_details": {
                "name": "127.0.0.1:12345 -> 127.0.0.1:5672",
                "peer_host": "127.0.0.1",
                "peer_port": 12345
            },
            "number": 1,
            "consumer_count": 2,
            "prefetch_count": 10,
            "messages_unacknowledged": 5,
            "message_stats": {
                "publish_details": { "rate": 12.5 },
                "deliver_get_details": { "rate": 8.0 },
                "ack_details": { "rate": 7.5 }
            }
        },
        {
            "name": "127.0.0.1:12345 -> 127.0.0.1:5672 (2)",
            "vhost": "/",
            "connection_details": {
                "name": "127.0.0.1:12345 -> 127.0.0.1:5672",
                "peer_host": "127.0.0.1",
                "peer_port": 12345
            },
            "number": 2,
            "consumer_count": 0,
            "prefetch_count": 0,
            "messages_unacknowledged": 0,
            "message_stats": null
        },
        {
            "name": "127.0.0.1:12346 -> 127.0.0.1:5672 (1)",
            "vhost": "/",
            "connection_details": {
                "name": "127.0.0.1:12346 -> 127.0.0.1:5672",
                "peer_host": "127.0.0.1",
                "peer_port": 12346
            },
            "number": 1,
            "consumer_count": 1,
            "prefetch_count": 5,
            "messages_unacknowledged": 1,
            "message_stats": {
                "publish_details": { "rate": 3.0 },
                "deliver_get_details": { "rate": 2.0 },
                "ack_details": { "rate": 1.5 }
            }
        }
    ]);

    let m = server
        .mock("GET", "/api/channels?page=1&page_size=50")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(body.to_string())
        .create();

    let client = client_for_mock(&url);
    let result = client
        .list_channels(
            Some("127.0.0.1:12345 -> 127.0.0.1:5672"),
            &Pagination { page: 1, page_size: 50 },
        )
        .await
        .expect("列出信道应成功");

    assert_eq!(result.items.len(), 2);
    assert_eq!(result.items[0].number, 1);
    assert_eq!(result.items[0].publish_rate, 12.5);
    assert_eq!(result.items[0].deliver_rate, 8.0);
    assert_eq!(result.items[0].ack_rate, 7.5);
    assert_eq!(result.items[1].number, 2);
    assert_eq!(result.items[1].publish_rate, 0.0);

    m.assert();
}

#[tokio::test]
async fn test_list_channels_pagination_params() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let m = server
        .mock("GET", "/api/channels?page=2&page_size=10")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body("[]")
        .create();

    let client = client_for_mock(&url);
    let result = client
        .list_channels(None, &Pagination { page: 2, page_size: 10 })
        .await
        .expect("分页列出信道应成功");

    assert!(result.items.is_empty());
    assert_eq!(result.page, 2);
    assert_eq!(result.page_size, 10);

    m.assert();
}
