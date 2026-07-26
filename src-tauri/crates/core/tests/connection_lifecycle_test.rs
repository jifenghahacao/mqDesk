//! 连接生命周期集成测试
//!
//! 复现 UI bug：create_connection 保存密码后，connect_to 读取失败，
//! 导致总览显示"未连接到 MQ"。

use mqdesk_core::models::Connection;
use mqdesk_core::state::AppState;
use mqdesk_core::storage::Storage;
use std::sync::Arc;
use tempfile::tempdir;

fn sample_connection() -> Connection {
    Connection {
        id: "conn-dev-001".to_string(),
        name: "dev".to_string(),
        host: "127.0.0.1".to_string(),
        amqp_port: 5672,
        management_port: 15672,
        management_scheme: "http".to_string(),
        vhost: "/".to_string(),
        username: "guest".to_string(),
        password: String::new(),
        created_at: "2026-07-25T00:00:00Z".to_string(),
        updated_at: "2026-07-25T00:00:00Z".to_string(),
    }
}

#[test]
fn create_save_password_then_connect_should_load_password() {
    let tmp = tempdir().expect("创建临时目录失败");
    let storage = Storage::open(tmp.path().to_path_buf()).expect("打开 sled 失败");

    let conn = sample_connection();
    let password = "guest";

    // 模拟 create_connection：先保存密码，再保存连接配置
    storage.save_password(&conn.id, password).expect("保存密码应成功");
    storage.upsert_connection(conn.clone()).expect("保存连接应成功");

    // 模拟 connect_to：读取连接 + 读取密码
    let loaded_conn = storage.get_connection(&conn.id).expect("读取连接应成功");
    let loaded_password = storage.load_password(&conn.id).expect("读取密码应成功");

    assert_eq!(loaded_conn.name, conn.name);
    assert_eq!(loaded_password, password, "connect_to 应能读到 create 时保存的密码");
}

#[test]
fn app_state_set_active_then_amqp_url_should_encode_vhost() {
    let tmp = tempdir().expect("创建临时目录失败");
    let storage = Storage::open(tmp.path().to_path_buf()).expect("打开 sled 失败");
    let state = Arc::new(AppState::with_storage(storage));

    let conn = sample_connection();
    state.set_active(conn, "guest".to_string());

    let active = state.get_active().expect("应存在活跃连接");
    assert_eq!(active.name, "dev");

    let url = state.amqp_url().expect("生成 AMQP URL 应成功");
    assert!(url.contains("%2F"), "默认 vhost / 应编码为 %2F，实际：{url}");
    assert!(url.starts_with("amqp://guest:guest@127.0.0.1:5672/"), "AMQP URL 格式错误：{url}");
}
