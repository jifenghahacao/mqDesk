// Tauri 命令调用封装
// 所有调用都通过 @tauri-apps/api/core 的 invoke 函数

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// === 连接管理 ===

export async function listConnections() {
  return invoke("list_connections");
}

export async function getConnection(id) {
  return invoke("get_connection", { id });
}

export async function createConnection(input) {
  return invoke("create_connection", { input });
}

export async function updateConnection(id, input) {
  return invoke("update_connection", { id, input });
}

export async function deleteConnection(id) {
  return invoke("delete_connection", { id });
}

export async function testConnection(input) {
  return invoke("test_connection", { input });
}

export async function connectTo(id) {
  return invoke("connect_to", { id });
}

export async function disconnect() {
  return invoke("disconnect");
}

export async function getActiveConnection() {
  return invoke("get_active_connection");
}

export async function getConnectionStatus(id) {
  return invoke("get_connection_status", { id });
}

export async function restoreLastActive() {
  return invoke("restore_last_active");
}

// === 集群节点 ===

export async function listNodes() {
  return invoke("list_nodes");
}

// === 消费者 ===

export async function listConsumers() {
  return invoke("list_consumers");
}

// === 总览 ===

export async function getOverview() {
  return invoke("get_overview");
}

// === 队列 ===

export async function listQueues(filter = {}) {
  return invoke("list_queues", { filter });
}

export async function getQueueDetail(name) {
  return invoke("get_queue_detail", { name });
}

export async function grabPreview(queueName, count = 10) {
  return invoke("grab_preview", { queueName, count });
}

export async function peekQueueMessages(queueName, count = 20) {
  return invoke("peek_queue_messages", { queueName, count });
}

export async function createQueue(config) {
  return invoke("create_queue", { config });
}

export async function updateQueuePolicy(input) {
  return invoke("update_queue_policy", { input });
}

export async function deleteQueue(name) {
  return invoke("delete_queue", { name });
}

export async function pauseQueue(name) {
  return invoke("pause_queue", { name });
}

export async function resumeQueue(name) {
  return invoke("resume_queue", { name });
}

export async function purgeQueue(name) {
  return invoke("purge_queue", { name });
}

export async function listQueueBindings(queueName) {
  return invoke("list_queue_bindings", { queueName });
}

export async function deleteQueueBinding(queueName, binding) {
  return invoke("delete_queue_binding", { queueName, binding });
}

export async function listQueuesPaginated(filter = {}, pagination = {}) {
  return invoke("list_queues_paginated", { filter, pagination });
}

// === RabbitMQ 连接与信道 ===

export async function listRabbitConnections(pagination = {}) {
  return invoke("list_rabbit_connections", { pagination });
}

export async function listChannels(connectionName = null, pagination = {}) {
  return invoke("list_channels", { connectionName, pagination });
}

// === 消息 ===

export async function publishMessage(request) {
  return invoke("publish_message", { request });
}

export async function listMessageFeed(filter = null) {
  return invoke("list_message_feed", { filter });
}

export async function getMessageTrace(traceId) {
  return invoke("get_message_trace", { traceId });
}

export async function deleteMessageTrace(traceId) {
  return invoke("delete_message_trace", { traceId });
}

// === 队列告警 ===

export async function setQueueAlertRule(rule) {
  return invoke("set_queue_alert_rule", { rule });
}

export async function listQueueAlertRules(queueName, vhost) {
  return invoke("list_queue_alert_rules", { queueName, vhost });
}

export async function deleteQueueAlertRule(queueName, vhost, metric) {
  return invoke("delete_queue_alert_rule", { queueName, vhost, metric });
}

export async function listQueueAlertRecords(queueName, vhost) {
  return invoke("list_queue_alert_records", { queueName, vhost });
}

export async function checkQueueAlerts() {
  return invoke("check_queue_alerts");
}

// === 审计日志 ===

export async function listQueueAuditLogs(filter = {}) {
  return invoke("list_queue_audit_logs", { filter });
}

export async function exportQueueAuditLogs(filter, path) {
  return invoke("export_queue_audit_logs", { filter, path });
}

// === 手动消费者（消费者工作室）===

export async function createManualConsumer(config) {
  return invoke("create_manual_consumer", { config });
}

export async function startManualConsumer(id) {
  return invoke("start_manual_consumer", { id });
}

export async function pauseManualConsumer(id) {
  return invoke("pause_manual_consumer", { id });
}

export async function resumeManualConsumer(id) {
  return invoke("resume_manual_consumer", { id });
}

export async function destroyManualConsumer(id) {
  return invoke("destroy_manual_consumer", { id });
}

export async function getManualConsumer(id) {
  return invoke("get_manual_consumer", { id });
}

export async function listManualConsumers() {
  return invoke("list_manual_consumers");
}

export async function listManualConsumerMessages(id, limit = 50) {
  return invoke("list_manual_consumer_messages", { id, limit });
}

export async function ackManualConsumerMessage(consumerId, messageId) {
  return invoke("ack_manual_consumer_message", { consumerId, messageId });
}

// === 窗口控制 ===

import { getCurrentWindow } from "@tauri-apps/api/window";

export async function minimizeWindow() {
  try {
    await getCurrentWindow().minimize();
  } catch (e) {
    console.warn("minimize failed:", e);
  }
}

export async function toggleMaximizeWindow() {
  try {
    const win = getCurrentWindow();
    await win.toggleMaximize();
  } catch (e) {
    console.warn("toggleMaximize failed:", e);
  }
}

export async function closeWindow() {
  try {
    await getCurrentWindow().close();
  } catch (e) {
    console.warn("close failed:", e);
  }
}

export async function startDragging() {
  try {
    await getCurrentWindow().startDragging();
  } catch (e) {
    console.warn("startDragging failed:", e);
  }
}

// === 自动刷新事件 ===

export async function listenQueueRefreshed(callback) {
  return listen("queue-refreshed", (event) => callback(event.payload));
}

export async function listenManagementStale(callback) {
  return listen("management-stale", (event) => callback(event.payload));
}

export async function setRefreshEnabled(enabled) {
  return invoke("set_refresh_enabled", { enabled });
}

export async function setRefreshInterval(ms) {
  return invoke("set_refresh_interval", { ms });
}

// === 错误处理 ===

export function extractErrorMessage(error) {
  if (typeof error === "string") return error;
  if (error?.message) return error.message;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}
