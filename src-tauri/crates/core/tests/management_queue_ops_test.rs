//! 队列操作 API 测试（Purge、Bindings 等）

use mqdesk_core::models::BindingInfo;
use mqdesk_core::rabbit::ManagementClient;

fn client_for_mock(server_url: &str) -> ManagementClient {
    let url = server_url.parse::<url::Url>().expect("mock URL 无效");
    let scheme = url.scheme();
    let host = url.host_str().unwrap_or("127.0.0.1");
    let port = url.port().unwrap_or(80);

    ManagementClient::new(scheme, host, port, "/", "guest", "guest")
}

#[tokio::test]
async fn test_purge_queue_success() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let m = server
        .mock("DELETE", "/api/queues/%2F/orders/contents")
        .with_status(204)
        .create();

    let client = client_for_mock(&url);
    client.purge_queue("orders").await.expect("purge 应成功");

    m.assert();
}

#[tokio::test]
async fn test_purge_queue_forbidden() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let m = server
        .mock("DELETE", "/api/queues/%2F/orders/contents")
        .with_status(403)
        .with_body("{\"error\":\"access_refused\"}")
        .create();

    let client = client_for_mock(&url);
    let result = client.purge_queue("orders").await;
    assert!(result.is_err());

    m.assert();
}

#[tokio::test]
async fn test_purge_queue_url_encoding() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let m = server
        .mock("DELETE", "/api/queues/%2F/my.queue%2Fwith%25special/contents")
        .with_status(204)
        .create();

    let client = client_for_mock(&url);
    client
        .purge_queue("my.queue/with%special")
        .await
        .expect("特殊字符队列名 purge 应成功");

    m.assert();
}

#[tokio::test]
async fn test_list_queue_bindings_success() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let body = serde_json::json!([
        {
            "source": "orders.exchange",
            "vhost": "/",
            "destination": "orders.queue",
            "destination_type": "queue",
            "routing_key": "orders.created",
            "arguments": {},
            "properties_key": "orders.created"
        },
        {
            "source": "",
            "vhost": "/",
            "destination": "orders.queue",
            "destination_type": "queue",
            "routing_key": "orders.queue",
            "arguments": {},
            "properties_key": "orders.queue"
        }
    ]);

    let m = server
        .mock("GET", "/api/queues/%2F/orders.queue/bindings")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(body.to_string())
        .create();

    let client = client_for_mock(&url);
    let bindings = client
        .list_queue_bindings("orders.queue")
        .await
        .expect("列出绑定应成功");

    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].source, "orders.exchange");
    assert_eq!(bindings[0].routing_key, "orders.created");
    assert_eq!(bindings[0].destination_type, "queue");
    assert_eq!(bindings[1].source, "");
    assert_eq!(bindings[1].routing_key, "orders.queue");

    m.assert();
}

#[tokio::test]
async fn test_delete_binding_success() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let m = server
        .mock("DELETE", "/api/bindings/%2F/e/orders.exchange/q/orders.queue/orders.created")
        .with_status(204)
        .create();

    let binding = BindingInfo {
        source: "orders.exchange".to_string(),
        vhost: "/".to_string(),
        destination: "orders.queue".to_string(),
        destination_type: "q".to_string(),
        routing_key: "orders.created".to_string(),
        arguments: serde_json::json!({}),
        properties_key: "orders.created".to_string(),
    };

    let client = client_for_mock(&url);
    client.delete_binding(&binding).await.expect("删除绑定应成功");

    m.assert();
}

#[tokio::test]
async fn test_delete_binding_url_encoding() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    let m = server
        .mock(
            "DELETE",
            "/api/bindings/%2F/e/my.ex%2Fchange/q/my.queue%2Fwith%25special/routing.key%2F%25test",
        )
        .with_status(204)
        .create();

    let binding = BindingInfo {
        source: "my.ex/change".to_string(),
        vhost: "/".to_string(),
        destination: "my.queue/with%special".to_string(),
        destination_type: "q".to_string(),
        routing_key: "routing.key/%test".to_string(),
        arguments: serde_json::json!({"x-arg": "value"}),
        properties_key: "routing.key/%test".to_string(),
    };

    let client = client_for_mock(&url);
    client
        .delete_binding(&binding)
        .await
        .expect("特殊字符绑定删除应成功");

    m.assert();
}
