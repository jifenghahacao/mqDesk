//! MQDesk — RabbitMQ 可视化管控台（Tauri 2.x 桌面壳）
//!
//! 项目结构：
//! - `error` — 统一错误类型，序列化为前端可读的 `AppError`
//! - `models` — 数据模型（连接、队列、消息、追踪）
//! - `storage` — sled 本地存储 + keyring 密码加密
//! - `rabbit` — RabbitMQ 客户端（Management HTTP API + AMQP publisher）
//! - `health` — 队列健康度四态判定
//! - `trace` — 消息状态追踪（推断式）
//! - `state` — 全局 AppState（连接管理 + RabbitMQClient）
//! - `commands` — Tauri 命令（连接/总览/队列/消息）
//!
//! MVP 范围按 PRD R1-R7 实现：
//! - R1 连接管理：新建/测试/保存/编辑/删除，密码本地加密
//! - R2 总览仪表盘：一句话健康度 + 4 统计卡 + 告警入口
//! - R3 队列列表：搜索/排序/健康色/行点击进详情
//! - R4 队列详情：健康色块 + 速率 SVG 图 + 抓取预览（requeue）
//! - R5 引导式发送：直发/交换机切换 + JSON 校验 + 二次确认 + 预判提示
//! - R6 健康度四态：正常/堆积预警/无人消费/空闲
//! - R7 消息通知列表：时间流 + 状态药丸 + 筛选 + 推断式追踪

mod commands;

use mqdesk_core::AppState;
use std::sync::Arc;
use tauri::Manager;

/// 将 PNG 字节解码为 Tauri 托盘图标可用的 RGBA Image
fn load_tray_icon(png_bytes: &[u8]) -> tauri::image::Image<'static> {
    let img = image::load_from_memory_with_format(png_bytes, image::ImageFormat::Png)
        .expect("托盘图标 PNG 解码失败");
    let rgba = img.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    tauri::image::Image::new_owned(rgba.into_raw(), width, height)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    let app_state = match AppState::new() {
        Ok(state) => Arc::new(state),
        Err(err) => {
            log::error!("初始化 AppState 失败：{err}");
            std::process::exit(1);
        }
    };

    let state_for_setup = Arc::clone(&app_state);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // 连接管理
            commands::connection::list_connections,
            commands::connection::get_connection,
            commands::connection::create_connection,
            commands::connection::update_connection,
            commands::connection::delete_connection,
            commands::connection::test_connection,
            commands::connection::connect_to,
            commands::connection::disconnect,
            commands::connection::get_active_connection,
            commands::connection::get_connection_status,
            commands::connection::restore_last_active,
            // 集群节点
            commands::node::list_nodes,
            // 消费者
            commands::consumer::list_consumers,
            commands::consumer::create_manual_consumer,
            commands::consumer::start_manual_consumer,
            commands::consumer::pause_manual_consumer,
            commands::consumer::resume_manual_consumer,
            commands::consumer::destroy_manual_consumer,
            commands::consumer::get_manual_consumer,
            commands::consumer::list_manual_consumers,
            commands::consumer::list_manual_consumer_messages,
            commands::consumer::ack_manual_consumer_message,
            // 总览
            commands::overview::get_overview,
            // 告警
            commands::alert::set_queue_alert_rule,
            commands::alert::list_queue_alert_rules,
            commands::alert::delete_queue_alert_rule,
            commands::alert::list_queue_alert_records,
            commands::alert::check_queue_alerts,
            // 审计
            commands::audit::list_queue_audit_logs,
            commands::audit::export_queue_audit_logs,
            // 队列
            commands::queue::list_queues,
            commands::queue::get_queue_detail,
            commands::queue::grab_preview,
            commands::queue::peek_queue_messages,
            commands::queue::create_queue,
            commands::queue::update_queue_policy,
            commands::queue::delete_queue,
            commands::queue::pause_queue,
            commands::queue::resume_queue,
            // 消息
            commands::message::publish_message,
            commands::message::list_message_feed,
            commands::message::get_message_trace,
            commands::message::delete_message_trace,
        ])
        .on_window_event(|window, event| {
            // 点击关闭按钮 → 隐藏窗口（最小化到托盘）
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            // 用 Tauri 的 async runtime 启动消息状态追踪后台任务
            // （不能用 tokio::spawn，因为 Tauri 环境下没有独立的 Tokio runtime）
            let state = state_for_setup.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = mqdesk_core::tokio::time::interval(std::time::Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    if let Err(e) = state.refresh_pending_status().await {
                        log::warn!("刷新消息状态失败：{e}");
                    }
                }
            });

            // 系统托盘图标（右键菜单：显示窗口 / 退出）
            let show_item = tauri::menu::MenuItemBuilder::with_id("show", "显示窗口")
                .build(app)?;
            let quit_item = tauri::menu::MenuItemBuilder::with_id("quit", "退出")
                .build(app)?;
            let menu = tauri::menu::MenuBuilder::new(app)
                .items(&[&show_item, &quit_item])
                .build()?;

            // 加载嵌入的托盘图标（避免 default_window_icon 在 dev 模式下为 None）
            let icon = load_tray_icon(include_bytes!("../icons/icon.png"));

            tauri::tray::TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .tooltip("MQDesk — RabbitMQ 可视化管控台")
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            log::info!("MQDesk 启动完成");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
