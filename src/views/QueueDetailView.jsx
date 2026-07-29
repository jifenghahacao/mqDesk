import { useEffect, useState } from "preact/hooks";
import { HealthBadge } from "../components/Badges.jsx";
import { BindingRow } from "../components/BindingRow.jsx";
import { ConfirmDialog } from "../components/ConfirmDialog.jsx";
import { QueueFormModal } from "../components/QueueFormModal.jsx";
import { RateChart } from "../components/RateChart.jsx";
import { Term } from "../components/Term.jsx";
import {
  checkQueueAlerts,
  deleteQueue,
  deleteQueueAlertRule,
  deleteQueueBinding,
  exportQueueAuditLogs,
  extractErrorMessage,
  getQueueDetail,
  listQueueAlertRecords,
  listQueueAlertRules,
  listQueueAuditLogs,
  listQueueBindings,
  pauseQueue,
  peekQueueMessages,
  purgeQueue,
  resumeQueue,
  setQueueAlertRule,
} from "../lib/api.js";
import { toastFail, toastOk } from "../lib/toast.js";

const TABS = [
  { key: "overview", label: "概览" },
  { key: "messages", label: "消息" },
  { key: "bindings", label: "绑定" },
  { key: "alerts", label: "告警" },
  { key: "audit", label: "审计" },
];

const METRIC_OPTIONS = [
  { value: "ready_count", label: "待消费消息数" },
  { value: "consumer_count", label: "消费者数" },
  { value: "incoming_rate", label: "流入速率（条/秒）" },
];

const OPERATOR_OPTIONS = [
  { value: "gt", label: "大于" },
  { value: "eq", label: "等于" },
  { value: "lt", label: "小于" },
];

const ACTION_LABELS = {
  create_queue: "创建队列",
  update_queue_policy: "更新策略",
  delete_queue: "删除队列",
  pause_queue: "暂停队列",
  resume_queue: "恢复队列",
};

function formatJson(value) {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

export function QueueDetailView({ queueName, onBack }) {
  const [detail, setDetail] = useState(null);
  const [messages, setMessages] = useState([]);
  const [selectedMessage, setSelectedMessage] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [tab, setTab] = useState(() => {
    const params = new URLSearchParams(window.location.search);
    const t = params.get("tab");
    return TABS.some((x) => x.key === t) ? t : "overview";
  });
  const [loadingMessages, setLoadingMessages] = useState(false);
  const [formOpen, setFormOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [purgeOpen, setPurgeOpen] = useState(false);
  const [operating, setOperating] = useState(false);

  // 绑定
  const [bindings, setBindings] = useState([]);
  const [loadingBindings, setLoadingBindings] = useState(false);

  // 告警
  const [rules, setRules] = useState([]);
  const [records, setRecords] = useState([]);
  const [loadingAlerts, setLoadingAlerts] = useState(false);
  const [alertMetric, setAlertMetric] = useState("ready_count");
  const [alertOperator, setAlertOperator] = useState("gt");
  const [alertThreshold, setAlertThreshold] = useState("");
  const [alertEnabled, setAlertEnabled] = useState(true);

  // 审计
  const [auditLogs, setAuditLogs] = useState([]);
  const [loadingAudit, setLoadingAudit] = useState(false);

  async function reloadDetail() {
    if (!queueName) return;
    setLoading(true);
    setError("");
    try {
      const data = await getQueueDetail(queueName);
      setDetail(data);
    } catch (e) {
      setError(extractErrorMessage(e));
    } finally {
      setLoading(false);
    }
  }

  async function reloadMessages() {
    if (!queueName) return;
    setLoadingMessages(true);
    try {
      const msgs = await peekQueueMessages(queueName, 20);
      setMessages(msgs);
    } catch (e) {
      toastFail(`加载消息失败：${extractErrorMessage(e)}`);
    } finally {
      setLoadingMessages(false);
    }
  }

  useEffect(() => {
    reloadDetail();
  }, [queueName]);

  useEffect(() => {
    if (tab === "messages") reloadMessages();
    if (tab === "bindings") reloadBindings();
    if (tab === "alerts") reloadAlerts();
    if (tab === "audit") reloadAudit();
  }, [tab]);

  async function reloadBindings() {
    if (!queueName) return;
    setLoadingBindings(true);
    try {
      const list = await listQueueBindings(queueName);
      setBindings(list);
    } catch (e) {
      toastFail(`加载绑定失败：${extractErrorMessage(e)}`);
    } finally {
      setLoadingBindings(false);
    }
  }

  async function handleDeleteBinding(binding) {
    if (!confirm(`确认解绑来源为「${binding.source || "-"}」、路由键为「${binding.routing_key || "-"}」的绑定？`)) {
      return;
    }
    try {
      await deleteQueueBinding(queueName, binding);
      toastOk("绑定已删除");
      reloadBindings();
    } catch (e) {
      toastFail(`解绑失败：${extractErrorMessage(e)}`);
    }
  }

  async function handlePurge() {
    setOperating(true);
    try {
      await purgeQueue(queueName);
      toastOk("队列已清空");
      setPurgeOpen(false);
      reloadDetail();
    } catch (e) {
      toastFail(`清空失败：${extractErrorMessage(e)}`);
    } finally {
      setOperating(false);
    }
  }

  async function reloadAlerts() {
    if (!detail) return;
    setLoadingAlerts(true);
    try {
      const [r, rec] = await Promise.all([
        listQueueAlertRules(queueName, detail.summary.vhost),
        listQueueAlertRecords(queueName, detail.summary.vhost),
      ]);
      setRules(r);
      setRecords(rec);
    } catch (e) {
      toastFail(`加载告警失败：${extractErrorMessage(e)}`);
    } finally {
      setLoadingAlerts(false);
    }
  }

  async function handleSaveAlert(e) {
    e.preventDefault();
    if (!detail) return;
    const threshold = Number(alertThreshold);
    if (Number.isNaN(threshold)) {
      toastFail("阈值必须是数字");
      return;
    }
    try {
      await setQueueAlertRule({
        queue_name: queueName,
        vhost: detail.summary.vhost,
        metric: alertMetric,
        operator: alertOperator,
        threshold,
        enabled: alertEnabled,
      });
      toastOk("告警规则已保存");
      setAlertThreshold("");
      reloadAlerts();
    } catch (err) {
      toastFail(`保存告警规则失败：${extractErrorMessage(err)}`);
    }
  }

  async function handleDeleteAlert(metric) {
    if (!detail) return;
    try {
      await deleteQueueAlertRule(queueName, detail.summary.vhost, metric);
      toastOk("告警规则已删除");
      reloadAlerts();
    } catch (err) {
      toastFail(`删除告警规则失败：${extractErrorMessage(err)}`);
    }
  }

  async function handleCheckAlerts() {
    try {
      const triggered = await checkQueueAlerts();
      if (triggered.length === 0) {
        toastOk("当前无新触发告警");
      } else {
        toastOk(`新触发 ${triggered.length} 条告警`);
      }
      reloadAlerts();
    } catch (err) {
      toastFail(`检查告警失败：${extractErrorMessage(err)}`);
    }
  }

  async function reloadAudit() {
    if (!detail) return;
    setLoadingAudit(true);
    try {
      const logs = await listQueueAuditLogs({ queue_name: queueName, vhost: detail.summary.vhost });
      setAuditLogs(logs);
    } catch (e) {
      toastFail(`加载审计日志失败：${extractErrorMessage(e)}`);
    } finally {
      setLoadingAudit(false);
    }
  }

  async function handleExportAudit() {
    if (!detail) return;
    const path = window.prompt("请输入导出文件路径（例如 /home/user/audit.json）：");
    if (!path) return;
    try {
      await exportQueueAuditLogs({ queue_name: queueName, vhost: detail.summary.vhost }, path);
      toastOk("审计日志已导出");
    } catch (e) {
      toastFail(`导出失败：${extractErrorMessage(e)}`);
    }
  }

  function copyPayload() {
    navigator.clipboard.writeText(selectedMessage.payload).then(
      () => toastOk("已复制 Payload"),
      () => toastFail("复制失败"),
    );
  }

  async function handlePause() {
    setOperating(true);
    try {
      await pauseQueue(queueName);
      toastOk("队列已暂停");
      reloadDetail();
    } catch (e) {
      toastFail(`暂停失败：${extractErrorMessage(e)}`);
    } finally {
      setOperating(false);
    }
  }

  async function handleResume() {
    setOperating(true);
    try {
      await resumeQueue(queueName);
      toastOk("队列已恢复");
      reloadDetail();
    } catch (e) {
      toastFail(`恢复失败：${extractErrorMessage(e)}`);
    } finally {
      setOperating(false);
    }
  }

  async function handleDelete() {
    setOperating(true);
    try {
      await deleteQueue(queueName);
      toastOk("队列已删除");
      setDeleteOpen(false);
      onBack();
    } catch (e) {
      toastFail(`删除失败：${extractErrorMessage(e)}`);
      setOperating(false);
    }
  }

  if (loading)
    return (
      <div class="view active">
        <div class="empty">加载中…</div>
      </div>
    );
  if (error) {
    return (
      <div class="view active">
        <div role="alert" class="banner danger">
          <div class="grow">
            <h2>无法加载队列详情</h2>
            <p>{error}</p>
          </div>
        </div>
        <button type="button" class="btn secondary" onClick={onBack} style="margin-top:16px;">
          ← 返回列表
        </button>
      </div>
    );
  }
  if (!detail) return null;

  const { summary } = detail;

  return (
    <section class="view active" data-view="queue-detail">
      <header class="page-head">
        <div>
          <button type="button" class="btn ghost sm" onClick={onBack}>
            ← 返回列表
          </button>
          <h1 style="margin-top:8px;">{summary.name}</h1>
          <p>
            vhost: {summary.vhost} · 类型: {summary.queue_type}
          </p>
        </div>
        <div class="actions">
          <button type="button" class="btn secondary" onClick={reloadDetail}>
            刷新
          </button>
          <button type="button" class="btn secondary" onClick={() => setFormOpen(true)}>
            编辑
          </button>
          <button type="button" class="btn warning" onClick={() => setPurgeOpen(true)} disabled={operating}>
            清空队列
          </button>
          {summary.queue_type === "classic" ? (
            <button type="button" class="btn secondary" onClick={handlePause} disabled={operating}>
              暂停
            </button>
          ) : null}
          {summary.queue_type === "classic" ? (
            <button type="button" class="btn secondary" onClick={handleResume} disabled={operating}>
              恢复
            </button>
          ) : null}
          <button type="button" class="btn danger" onClick={() => setDeleteOpen(true)} disabled={operating}>
            删除
          </button>
        </div>
      </header>

      <div class={`banner ${summary.health}`}>
        <span class={`status-dot ${summary.health}`} />
        <div class="grow">
          <h2>
            <HealthBadge status={summary.health} />
          </h2>
          <p>{summary.health_summary}</p>
        </div>
      </div>

      <div class="seg qd-tabs" style="margin-bottom:16px;">
        {TABS.map((t) => (
          <button key={t.key} type="button" class={tab === t.key ? "on" : ""} onClick={() => setTab(t.key)}>
            {t.label}
          </button>
        ))}
      </div>

      {tab === "overview" ? (
        <>
          <div class="qd-stats">
            <div class="qd-stat">
              <div class="label">
                <Term termKey="ready" label="待消费" />
              </div>
              <div class="num">{summary.ready.toLocaleString()}</div>
            </div>
            <div class="qd-stat">
              <div class="label">
                <Term termKey="unacked" label="处理中" />
              </div>
              <div class="num">{summary.unacked.toLocaleString()}</div>
            </div>
            <div class="qd-stat">
              <div class="label">总数</div>
              <div class="num">{summary.total.toLocaleString()}</div>
            </div>
            <div class="qd-stat">
              <div class="label">
                <Term termKey="consumer" label="消费者" />
              </div>
              <div class="num">{summary.consumers}</div>
              <div class="rate">
                进 {summary.incoming_rate.toFixed(1)} <small>/ 出 {summary.outgoing_rate.toFixed(1)}</small>
              </div>
            </div>
          </div>

          <div class="card">
            <div class="col-title">
              <h3>近 12 个采样点的速率</h3>
            </div>
            <RateChart incoming={detail.rate_history.incoming} outgoing={detail.rate_history.outgoing} />
          </div>

          <div class="cols" style="margin-top:16px;">
            <div class="card">
              <div class="col-title">
                <h3>队列配置参数</h3>
              </div>
              <pre class="json-block">{formatJson(detail.arguments)}</pre>
              {detail.policy ? <p class="muted">Policy: {detail.policy}</p> : null}
            </div>

            <div class="card">
              <div class="col-title">
                <h3>消费者连接</h3>
              </div>
              {detail.consumers.length === 0 ? (
                <div class="empty">暂无消费者连接</div>
              ) : (
                <ul class="connection-list">
                  {detail.consumers.map((c) => (
                    <li key={c.name} class="connection-item">
                      <div class="ci-name">{c.name}</div>
                      <div class="ci-meta">
                        {c.peer_address} · Ack: {c.ack_required ? "是" : "否"} · Prefetch: {c.prefetch_count}
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        </>
      ) : null}

      {tab === "messages" ? (
        <div class="card">
          <div class="col-title">
            <h3>消息列表（只看不消费）</h3>
            <button type="button" class="link" onClick={reloadMessages}>
              刷新
            </button>
          </div>
          {loadingMessages ? (
            <div class="empty">加载中…</div>
          ) : messages.length === 0 ? (
            <div class="empty">没有可预览的消息。</div>
          ) : (
            <div class="table-wrap">
              <table class="tbl message-table">
                <thead>
                  <tr>
                    <th>Delivery Tag</th>
                    <th>Exchange</th>
                    <th>
                      <Term termKey="routing_key" label="路由键" />
                    </th>
                    <th>大小</th>
                    <th>操作</th>
                  </tr>
                </thead>
                <tbody>
                  {messages.map((m) => (
                    <tr key={m.delivery_tag} class="queue-row" onClick={() => setSelectedMessage(m)}>
                      <td class="mono">{m.delivery_tag}</td>
                      <td class="mono">{m.exchange || "-"}</td>
                      <td class="mono">{m.routing_key}</td>
                      <td class="mono">{m.payload_size} B</td>
                      <td>
                        <button
                          type="button"
                          class="btn sm ghost"
                          onClick={(e) => {
                            e.stopPropagation();
                            setSelectedMessage(m);
                          }}
                        >
                          详情
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      ) : null}

      {tab === "bindings" ? (
        <div class="card">
          <div class="col-title">
            <h3>绑定规则</h3>
            <button type="button" class="link" onClick={reloadBindings}>
              刷新
            </button>
          </div>
          {loadingBindings ? (
            <div class="empty">加载中…</div>
          ) : bindings.length === 0 ? (
            <div class="empty">暂无绑定规则。</div>
          ) : (
            <div class="table-wrap">
              <table class="tbl binding-table">
                <thead>
                  <tr>
                    <th>来源 Exchange</th>
                    <th>路由键</th>
                    <th>目标类型</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {bindings.map((b) => (
                    <BindingRow key={`${b.source}-${b.routing_key}`} binding={b} onDelete={handleDeleteBinding} />
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      ) : null}

      {tab === "alerts" ? (
        <div class="card">
          <div class="col-title">
            <h3>告警规则</h3>
            <button type="button" class="link" onClick={handleCheckAlerts}>
              立即检查
            </button>
          </div>

          <form class="alert-form" onSubmit={handleSaveAlert}>
            <select class="input" value={alertMetric} onChange={(e) => setAlertMetric(e.target.value)}>
              {METRIC_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </select>
            <select class="input" value={alertOperator} onChange={(e) => setAlertOperator(e.target.value)}>
              {OPERATOR_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </select>
            <input
              class="input"
              type="number"
              step="any"
              placeholder="阈值"
              value={alertThreshold}
              onInput={(e) => setAlertThreshold(e.target.value)}
              required
            />
            <label class="checkbox">
              <input type="checkbox" checked={alertEnabled} onChange={(e) => setAlertEnabled(e.target.checked)} />
              启用
            </label>
            <button type="submit" class="btn primary sm">
              保存规则
            </button>
          </form>

          {loadingAlerts ? (
            <div class="empty">加载中…</div>
          ) : rules.length === 0 ? (
            <div class="empty">暂无告警规则。</div>
          ) : (
            <div class="table-wrap" style="margin-top:16px;">
              <table class="tbl alert-table">
                <thead>
                  <tr>
                    <th>指标</th>
                    <th>运算符</th>
                    <th>阈值</th>
                    <th>状态</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {rules.map((r) => (
                    <tr key={r.metric}>
                      <td>{METRIC_OPTIONS.find((o) => o.value === r.metric)?.label || r.metric}</td>
                      <td>{OPERATOR_OPTIONS.find((o) => o.value === r.operator)?.label || r.operator}</td>
                      <td class="mono">{r.threshold}</td>
                      <td>
                        <span class={`pill ${r.enabled ? "ok" : "idle"}`}>{r.enabled ? "启用" : "停用"}</span>
                      </td>
                      <td>
                        <button type="button" class="btn sm ghost" onClick={() => handleDeleteAlert(r.metric)}>
                          删除
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          <div class="col-title" style="margin-top:24px;">
            <h3>告警记录</h3>
          </div>
          {records.length === 0 ? (
            <div class="empty">暂无告警记录。</div>
          ) : (
            <div class="table-wrap">
              <table class="tbl alert-table">
                <thead>
                  <tr>
                    <th>时间</th>
                    <th>指标</th>
                    <th>阈值</th>
                    <th>实际值</th>
                    <th>状态</th>
                  </tr>
                </thead>
                <tbody>
                  {records.map((rec) => (
                    <tr key={rec.id}>
                      <td class="mono">{new Date(rec.triggered_at).toLocaleString()}</td>
                      <td>{METRIC_OPTIONS.find((o) => o.value === rec.metric)?.label || rec.metric}</td>
                      <td class="mono">{rec.threshold}</td>
                      <td class="mono">{rec.actual_value.toFixed(2)}</td>
                      <td>
                        <span class={`pill ${rec.resolved_at ? "ok" : "danger"}`}>
                          {rec.resolved_at ? "已恢复" : "触发中"}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      ) : null}

      {tab === "audit" ? (
        <div class="card">
          <div class="col-title">
            <h3>审计日志</h3>
            <div style="display:flex; gap:12px;">
              <button type="button" class="link" onClick={reloadAudit}>
                刷新
              </button>
              <button type="button" class="link" onClick={handleExportAudit}>
                导出
              </button>
            </div>
          </div>
          {loadingAudit ? (
            <div class="empty">加载中…</div>
          ) : auditLogs.length === 0 ? (
            <div class="empty">暂无审计日志。</div>
          ) : (
            <div class="table-wrap">
              <table class="tbl audit-table">
                <thead>
                  <tr>
                    <th>时间</th>
                    <th>操作</th>
                    <th>操作人</th>
                    <th>详情</th>
                  </tr>
                </thead>
                <tbody>
                  {auditLogs.map((log) => (
                    <tr key={log.id}>
                      <td class="mono">{new Date(log.timestamp).toLocaleString()}</td>
                      <td>{ACTION_LABELS[log.action] || log.action}</td>
                      <td>{log.operator}</td>
                      <td class="mono" style="max-width:320px;">
                        {log.detail}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      ) : null}

      {selectedMessage ? (
        <div class="scrim show" onClick={() => setSelectedMessage(null)}>
          <div class="modal show" onClick={(e) => e.stopPropagation()}>
            <h3>消息详情</h3>
            <div class="field">
              <label>Delivery Tag</label>
              <input class="input" value={selectedMessage.delivery_tag} readOnly />
            </div>
            <div class="field">
              <label>Exchange</label>
              <input class="input" value={selectedMessage.exchange || "-"} readOnly />
            </div>
            <div class="field">
              <label>
                <Term termKey="routing_key" label="路由键" />
              </label>
              <input class="input" value={selectedMessage.routing_key} readOnly />
            </div>
            <div class="field">
              <label>Payload</label>
              <pre class="json-block payload-block">{formatJson(selectedMessage.payload)}</pre>
            </div>
            <div class="field">
              <label>Headers</label>
              <pre class="json-block">{formatJson(selectedMessage.headers)}</pre>
            </div>
            <div class="modal-actions">
              <button type="button" class="btn secondary" onClick={() => setSelectedMessage(null)}>
                关闭
              </button>
              <button type="button" class="btn primary" onClick={copyPayload}>
                复制 Payload
              </button>
            </div>
          </div>
        </div>
      ) : null}

      <QueueFormModal open={formOpen} queue={detail} onClose={() => setFormOpen(false)} onSaved={reloadDetail} />

      <ConfirmDialog
        open={deleteOpen}
        title="确认删除队列"
        desc={`即将删除队列 <b>${summary.name}</b>（当前共 ${summary.total} 条消息）。此操作不可恢复。`}
        danger
        okText="确认删除"
        onOk={handleDelete}
        onCancel={() => setDeleteOpen(false)}
      />

      <ConfirmDialog
        open={purgeOpen}
        title="确认清空队列"
        desc={`即将清空队列 <b>${summary.name}</b> 中的所有消息（当前共 ${summary.total} 条）。此操作不可恢复。`}
        danger
        okText="确认清空"
        onOk={handlePurge}
        onCancel={() => setPurgeOpen(false)}
      />
    </section>
  );
}
