//! RabbitMQ 队列列表分页测试

use mqdesk_core::models::{Pagination, QueueFilter};
use mqdesk_core::rabbit::ManagementClient;

fn client_for_mock(server_url: &str) -> ManagementClient {
    let url = server_url.parse::<url::Url>().expect("mock URL 无效");
    let scheme = url.scheme();
    let host = url.host_str().unwrap_or("127.0.0.1");
    let port = url.port().unwrap_or(80);

    ManagementClient::new(scheme, host, port, "/", "guest", "guest")
}

#[tokio::test]
async fn test_list_queues_pagination_success() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let body = serde_json::json!([
        {
            "name": "queue-1",
            "vhost": "/",
            "type": "classic",
            "durable": true,
            "messages": 10,
            "messages_ready": 10,
            "messages_unacknowledged": 0,
            "consumers": 0
        },
        {
            "name": "queue-2",
            "vhost": "/",
            "type": "quorum",
            "durable": true,
            "messages": 0,
            "messages_ready": 0,
            "messages_unacknowledged": 0,
            "consumers": 2
        }
    ]);

    let m = server
        .mock("GET", "/api/queues/%2F?page=1&page_size=50")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(body.to_string())
        .create();

    let client = client_for_mock(&url);
    let result = client
        .list_queues(&QueueFilter::default(), &Pagination { page: 1, page_size: 50 })
        .await
        .expect("列出队列应成功");

    assert_eq!(result.items.len(), 2);
    assert_eq!(result.total, 2);
    assert_eq!(result.page, 1);
    assert_eq!(result.page_size, 50);
    assert_eq!(result.items[0].name, "queue-1");
    assert_eq!(result.items[1].queue_type, "quorum");

    m.assert();
}

#[tokio::test]
async fn test_list_queues_pagination_params() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let m = server
        .mock("GET", "/api/queues/%2F?page=2&page_size=10")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body("[]")
        .create();

    let client = client_for_mock(&url);
    let result = client
        .list_queues(&QueueFilter::default(), &Pagination { page: 2, page_size: 10 })
        .await
        .expect("分页列出队列应成功");

    assert!(result.items.is_empty());
    assert_eq!(result.page, 2);
    assert_eq!(result.page_size, 10);

    m.assert();
}

#[tokio::test]
async fn test_list_queues_filter_with_pagination() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let body = serde_json::json!([
        { "name": "orders-a", "vhost": "/", "type": "classic", "messages": 0 },
        { "name": "logs-b", "vhost": "/", "type": "classic", "messages": 0 },
        { "name": "orders-c", "vhost": "/", "type": "quorum", "messages": 0 }
    ]);

    let m = server
        .mock("GET", "/api/queues/%2F?page=1&page_size=50")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(body.to_string())
        .create();

    let client = client_for_mock(&url);
    let filter = QueueFilter {
        search: "orders".to_string(),
        queue_type: String::new(),
        health: String::new(),
    };
    let result = client
        .list_queues(&filter, &Pagination { page: 1, page_size: 50 })
        .await
        .expect("过滤列出队列应成功");

    assert_eq!(result.items.len(), 2);
    assert!(result.items.iter().all(|q| q.name.starts_with("orders")));

    m.assert();
}
