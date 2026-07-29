//! 连接管理命令

use mqdesk_core::error::{AppError, AppResult};
use mqdesk_core::models::{Connection, ConnectionInfo, ConnectionInput, ConnectionStatus, Paginated, Pagination};
use mqdesk_core::state::AppState;
use mqdesk_core::storage::now_iso;
use mqdesk_core::uuid::Uuid;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn list_connections(state: State<'_, Arc<AppState>>) -> AppResult<Vec<Connection>> {
    let mut list = state.storage.list_connections()?;
    // 不暴露密码
    for c in &mut list {
        c.password = String::new();
    }
    Ok(list)
}

#[tauri::command]
pub async fn get_connection(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Connection> {
    let mut conn = state.storage.get_connection(&id)?;
    conn.password = String::new();
    Ok(conn)
}

#[tauri::command]
pub async fn create_connection(
    input: ConnectionInput,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Connection> {
    validate_input(&input)?;
    let id = Uuid::new_v4().to_string();
    let now = now_iso();
    let conn = Connection {
        id,
        name: input.name,
        host: input.host,
        amqp_port: input.amqp_port,
        management_port: input.management_port,
        management_scheme: input.management_scheme,
        vhost: input.vhost,
        username: input.username,
        password: String::new(), // 不存明文
        created_at: now.clone(),
        updated_at: now,
    };
    state.storage.save_password(&conn.id, &input.password)?;
    state.storage.upsert_connection(conn.clone())?;
    let mut result = conn;
    result.password = String::new();
    Ok(result)
}

#[tauri::command]
pub async fn update_connection(
    id: String,
    input: ConnectionInput,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Connection> {
    validate_input(&input)?;
    let mut existing = state.storage.get_connection(&id)?;
    existing.name = input.name;
    existing.host = input.host;
    existing.amqp_port = input.amqp_port;
    existing.management_port = input.management_port;
    existing.management_scheme = input.management_scheme;
    existing.vhost = input.vhost;
    existing.username = input.username;
    existing.updated_at = now_iso();

    if !input.password.is_empty() {
        state.storage.save_password(&existing.id, &input.password)?;
    }
    state.storage.upsert_connection(existing.clone())?;
    let mut result = existing;
    result.password = String::new();
    Ok(result)
}

#[tauri::command]
pub async fn delete_connection(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<()> {
    // 如果删除的是当前活跃连接，先断开
    {
        let active = state.active_connection.read();
        if let Some(a) = active.as_ref() {
            if a.connection.id == id {
                drop(active);
                state.clear_active();
            }
        }
    }
    state.storage.delete_connection(&id)?;
    Ok(())
}

#[tauri::command]
pub async fn test_connection(input: ConnectionInput) -> AppResult<String> {
    validate_input(&input)?;
    let client = mqdesk_core::rabbit::ManagementClient::new(
        &input.management_scheme,
        &input.host,
        input.management_port,
        &input.vhost,
        &input.username,
        &input.password,
    );
    let name = client.test_connection().await?;
    Ok(format!("已连接：{name}"))
}

#[tauri::command]
pub async fn connect_to(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Connection> {
    let conn = state.storage.get_connection(&id)?;
    let password = state.storage.load_password(&conn.id)?;
    state.set_active(conn.clone(), password);
    let mut result = conn;
    result.password = String::new();
    Ok(result)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, Arc<AppState>>) -> AppResult<()> {
    state.clear_active();
    Ok(())
}

#[tauri::command]
pub async fn get_active_connection(state: State<'_, Arc<AppState>>) -> AppResult<Option<Connection>> {
    Ok(state.get_active())
}

/// 获取某个连接的实时状态（是否当前活跃、Management API 是否可达）
#[tauri::command]
pub async fn get_connection_status(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<ConnectionStatus> {
    let conn = state.storage.get_connection(&id)?;
    let password = state.storage.load_password(&id).unwrap_or_default();

    let is_active = state
        .active_connection
        .read()
        .as_ref()
        .map(|a| a.connection.id == id)
        .unwrap_or(false);

    let client = mqdesk_core::rabbit::ManagementClient::new(
        &conn.management_scheme,
        &conn.host,
        conn.management_port,
        &conn.vhost,
        &conn.username,
        &password,
    );

    let (is_reachable, cluster_name, error) = match client.test_connection().await {
        Ok(name) => (true, Some(name), None),
        Err(e) => (false, None, Some(e.to_string())),
    };

    Ok(ConnectionStatus {
        id: conn.id,
        name: conn.name,
        host: conn.host,
        management_port: conn.management_port,
        vhost: conn.vhost,
        username: conn.username,
        is_active,
        is_reachable,
        cluster_name,
        error,
    })
}

/// 启动时恢复上次活跃连接（仅恢复内存状态，不测试连通性）
#[tauri::command]
pub async fn restore_last_active(state: State<'_, Arc<AppState>>) -> AppResult<Option<Connection>> {
    state.restore_last_active()
}

/// 列出当前活跃 RabbitMQ 连接（与本地保存的连接配置区分）
#[tauri::command]
pub async fn list_rabbit_connections(
    pagination: Option<Pagination>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Paginated<ConnectionInfo>> {
    if state.get_active().is_none() {
        return Err(AppError::NotConnected);
    }
    let pagination = pagination.unwrap_or_default();
    let management = state.rabbit_management();
    management.list_connections(&pagination).await
}

fn validate_input(input: &ConnectionInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::InvalidConnection("名称不能为空".to_string()));
    }
    if input.host.trim().is_empty() {
        return Err(AppError::InvalidConnection("主机地址不能为空".to_string()));
    }
    if input.username.trim().is_empty() {
        return Err(AppError::InvalidConnection("用户名不能为空".to_string()));
    }
    if input.password.is_empty() {
        // 测试连接允许空密码提示
        return Err(AppError::InvalidConnection("密码不能为空".to_string()));
    }
    if !matches!(input.management_scheme.as_str(), "http" | "https") {
        return Err(AppError::InvalidConnection(
            "管理接口协议必须是 http 或 https".to_string(),
        ));
    }
    Ok(())
}
