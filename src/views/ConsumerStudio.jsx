import { useEffect, useMemo, useState } from "preact/hooks";
import { SearchableSelect } from "../components/SearchableSelect.jsx";
import {
  ackManualConsumerMessage,
  createManualConsumer,
  destroyManualConsumer,
  extractErrorMessage,
  listManualConsumerMessages,
  listManualConsumers,
  listQueues,
  pauseManualConsumer,
  resumeManualConsumer,
  startManualConsumer,
} from "../lib/api.js";
import { toastFail, toastOk } from "../lib/toast.js";

const EMPTY_FORM = {
  name: "",
  queue_name: "",
  mode: "async",
  prefetch_count: 10,
  auto_ack: false,
  filter: {
    payload_type: "contains",
    payload_value: "",
    headers: [],
  },
};

const STATUS_LABEL = {
  pending: "待启动",
  running: "运行中",
  paused: "已暂停",
  destroyed: "已销毁",
  error: "错误",
};

const STATUS_CLASS = {
  pending: "idle",
  running: "ok",
  paused: "warn",
  destroyed: "danger",
  error: "danger",
};

function formatBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function formatTime(ts) {
  const d = new Date(ts);
  return d.toLocaleTimeString("zh-CN", { hour12: false }) + "." + String(d.getMilliseconds()).padStart(3, "0");
}

export function ConsumerStudio() {
  const [form, setForm] = useState(EMPTY_FORM);
  const [queues, setQueues] = useState([]);
  const [consumers, setConsumers] = useState([]);
  const [selectedId, setSelectedId] = useState(null);
  const [messages, setMessages] = useState([]);
  const [loadingQueues, setLoadingQueues] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState("");

  const selectedConsumer = useMemo(() => consumers.find((c) => c.id === selectedId) || null, [consumers, selectedId]);

  async function loadQueues() {
    setLoadingQueues(true);
    try {
      const list = await listQueues();
      setQueues(list);
      if (list.length > 0 && !form.queue_name) {
        setForm((prev) => ({ ...prev, queue_name: list[0].name }));
      }
    } catch (e) {
      console.warn("加载队列失败", e);
    } finally {
      setLoadingQueues(false);
    }
  }

  async function loadConsumers() {
    try {
      const list = await listManualConsumers();
      setConsumers(list);
      if (selectedId && !list.find((c) => c.id === selectedId)) {
        setSelectedId(null);
      }
    } catch (e) {
      console.warn("加载消费者失败", e);
    }
  }

  async function loadMessages() {
    if (!selectedId) {
      setMessages([]);
      return;
    }
    try {
      const list = await listManualConsumerMessages(selectedId, 100);
      setMessages(list);
    } catch (e) {
      console.warn("加载消息失败", e);
    }
  }

  useEffect(() => {
    loadQueues();
  }, []);

  useEffect(() => {
    loadConsumers();
    const id = setInterval(loadConsumers, 2000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    loadMessages();
    const id = setInterval(loadMessages, 1500);
    return () => clearInterval(id);
  }, [selectedId]);

  function updateField(field, value) {
    setForm((prev) => ({ ...prev, [field]: value }));
  }

  function updateFilterField(field, value) {
    setForm((prev) => ({
      ...prev,
      filter: { ...prev.filter, [field]: value },
    }));
  }

  function addHeaderFilter() {
    setForm((prev) => ({
      ...prev,
      filter: {
        ...prev.filter,
        headers: [...prev.filter.headers, { key: "", value: "" }],
      },
    }));
  }

  function updateHeaderFilter(index, key, value) {
    setForm((prev) => {
      const headers = prev.filter.headers.map((h, i) => (i === index ? { ...h, [key]: value } : h));
      return { ...prev, filter: { ...prev.filter, headers } };
    });
  }

  function removeHeaderFilter(index) {
    setForm((prev) => ({
      ...prev,
      filter: {
        ...prev.filter,
        headers: prev.filter.headers.filter((_, i) => i !== index),
      },
    }));
  }

  function validate() {
    if (!form.name.trim()) {
      toastFail("请输入消费者名称");
      return false;
    }
    if (!form.queue_name) {
      toastFail("请选择队列");
      return false;
    }
    if (form.prefetch_count < 1 || form.prefetch_count > 1000) {
      toastFail("预取值应在 1-1000 之间");
      return false;
    }
    return true;
  }

  async function handleCreate() {
    if (!validate()) return;
    setSubmitting(true);
    setError("");
    try {
      const config = {
        ...form,
        prefetch_count: Number(form.prefetch_count),
      };
      const consumer = await createManualConsumer(config);
      setConsumers((prev) => [...prev, consumer]);
      setSelectedId(consumer.id);
      setForm(EMPTY_FORM);
      toastOk("消费者创建成功");
      loadQueues();
    } catch (e) {
      const msg = extractErrorMessage(e);
      setError(msg);
      toastFail(`创建失败：${msg}`);
    } finally {
      setSubmitting(false);
    }
  }

  async function handleStart(id) {
    try {
      const c = await startManualConsumer(id);
      setConsumers((prev) => prev.map((x) => (x.id === id ? c : x)));
      toastOk("开始消费");
    } catch (e) {
      toastFail(`启动失败：${extractErrorMessage(e)}`);
    }
  }

  async function handlePause(id) {
    try {
      const c = await pauseManualConsumer(id);
      setConsumers((prev) => prev.map((x) => (x.id === id ? c : x)));
      toastOk("已暂停");
    } catch (e) {
      toastFail(`暂停失败：${extractErrorMessage(e)}`);
    }
  }

  async function handleResume(id) {
    try {
      const c = await resumeManualConsumer(id);
      setConsumers((prev) => prev.map((x) => (x.id === id ? c : x)));
      toastOk("已继续");
    } catch (e) {
      toastFail(`继续失败：${extractErrorMessage(e)}`);
    }
  }

  async function handleDestroy(id) {
    if (!confirm("确定要销毁该消费者吗？未确认的消息将重新入队。")) return;
    try {
      await destroyManualConsumer(id);
      setConsumers((prev) => prev.filter((x) => x.id !== id));
      if (selectedId === id) setSelectedId(null);
      toastOk("已销毁");
    } catch (e) {
      toastFail(`销毁失败：${extractErrorMessage(e)}`);
    }
  }

  async function handleAck(messageId) {
    if (!selectedId) return;
    try {
      await ackManualConsumerMessage(selectedId, messageId);
      setMessages((prev) => prev.map((m) => (m.id === messageId ? { ...m, acked: true } : m)));
      toastOk("已确认消费");
    } catch (e) {
      toastFail(`确认失败：${extractErrorMessage(e)}`);
    }
  }

  return (
    <section class="view active" data-view="consumer-studio">
      <header class="page-head">
        <div>
          <h1>消费者工作室</h1>
          <p>手动创建消费者，默认只看不消费（预览模式），可逐条确认</p>
        </div>
      </header>

      {error ? (
        <div role="alert" class="banner danger">
          <div class="grow">
            <p>{error}</p>
          </div>
        </div>
      ) : null}

      <div class="cs-layout">
        <div class="cs-form card">
          <h3>创建消费者</h3>

          <div class="field">
            <label>消费者名称</label>
            <input
              class="input"
              value={form.name}
              onInput={(e) => updateField("name", e.target.value)}
              placeholder="例如：订单测试消费者"
            />
          </div>

          <div class="field">
            <label>目标队列</label>
            <SearchableSelect
              value={form.queue_name}
              options={queues.map((q) => q.name)}
              onChange={(value) => updateField("queue_name", value)}
              placeholder={loadingQueues ? "加载中…" : "请选择队列"}
              disabled={loadingQueues}
            />
          </div>

          <div class="field">
            <label>处理模式</label>
            <div class="seg">
              <button
                type="button"
                class={form.mode === "sync" ? "on" : ""}
                onClick={() => updateField("mode", "sync")}
              >
                同步
              </button>
              <button
                type="button"
                class={form.mode === "async" ? "on" : ""}
                onClick={() => updateField("mode", "async")}
              >
                异步
              </button>
            </div>
          </div>

          {form.mode === "async" ? (
            <div class="field">
              <label>预取值（prefetch）</label>
              <input
                class="input"
                type="number"
                min="1"
                max="1000"
                value={form.prefetch_count}
                onInput={(e) => updateField("prefetch_count", e.target.value)}
              />
              <p class="hint">单次从 RabbitMQ 取回的最大未确认消息数，建议 1-50。</p>
            </div>
          ) : null}

          <div class="field">
            <label>Ack 模式</label>
            <div class="seg">
              <button type="button" class={!form.auto_ack ? "on" : ""} onClick={() => updateField("auto_ack", false)}>
                手动 Ack（预览，只看不消费）
              </button>
              <button type="button" class={form.auto_ack ? "on" : ""} onClick={() => updateField("auto_ack", true)}>
                自动 Ack（真实消费）
              </button>
            </div>
            <p class="hint">手动 Ack 模式下，消息进入列表但仍在队列中；点「确认」才真正消费。</p>
          </div>

          <div class="field">
            <label>Payload 过滤</label>
            <div class="flex" style="gap:8px;">
              <select
                class="input"
                style="width:120px;flex:none"
                value={form.filter.payload_type}
                onChange={(e) => updateFilterField("payload_type", e.target.value)}
              >
                <option value="contains">包含</option>
                <option value="equals">等于</option>
                <option value="regex">正则</option>
              </select>
              <input
                class="input"
                value={form.filter.payload_value}
                onInput={(e) => updateFilterField("payload_value", e.target.value)}
                placeholder="留空表示不过滤"
              />
            </div>
            <p class="hint">仅让符合条件的消息进入列表；不满足的消息会重新入队。正则模式使用 Rust 语法。</p>
          </div>

          <div class="field">
            <label>Header 过滤</label>
            {form.filter.headers.map((h, i) => (
              <div key={i} class="flex" style="gap:8px;marginBottom:8px">
                <input
                  class="input"
                  placeholder="key"
                  value={h.key}
                  onInput={(e) => updateHeaderFilter(i, "key", e.target.value)}
                />
                <input
                  class="input"
                  placeholder="value"
                  value={h.value}
                  onInput={(e) => updateHeaderFilter(i, "value", e.target.value)}
                />
                <button type="button" class="btn ghost" onClick={() => removeHeaderFilter(i)}>
                  删除
                </button>
              </div>
            ))}
            <button type="button" class="btn sm ghost" onClick={addHeaderFilter}>
              + 添加 Header 条件
            </button>
          </div>

          <div class="actions">
            <button type="button" class="btn primary" onClick={handleCreate} disabled={submitting}>
              {submitting ? "创建中…" : "创建消费者"}
            </button>
          </div>
        </div>

        <div class="cs-main">
          <div class="card">
            <h3>消费者列表</h3>
            {consumers.length === 0 ? (
              <div class="empty">暂无消费者，请在左侧创建。</div>
            ) : (
              <table class="tbl">
                <thead>
                  <tr>
                    <th>名称</th>
                    <th>队列</th>
                    <th>模式</th>
                    <th>Ack</th>
                    <th>状态</th>
                    <th>已接收</th>
                    <th>已过滤</th>
                    <th>操作</th>
                  </tr>
                </thead>
                <tbody>
                  {consumers.map((c) => (
                    <tr
                      key={c.id}
                      class={selectedId === c.id ? "selected-row" : ""}
                      onClick={() => setSelectedId(c.id)}
                    >
                      <td>{c.name}</td>
                      <td class="mono">{c.queue_name}</td>
                      <td>{c.mode === "async" ? "异步" : "同步"}</td>
                      <td>{c.auto_ack ? "自动" : "手动"}</td>
                      <td>
                        <span class={`pill ${STATUS_CLASS[c.status] || "idle"}`}>
                          {STATUS_LABEL[c.status] || c.status}
                        </span>
                        {c.error ? (
                          <span class="muted" style="margin-left:6px">
                            ({c.error})
                          </span>
                        ) : null}
                      </td>
                      <td>{c.consumed_count}</td>
                      <td>{c.filtered_count}</td>
                      <td>
                        <div class="flex" style="gap:6px">
                          {c.status === "pending" || c.status === "paused" ? (
                            <button
                              type="button"
                              class="btn sm primary"
                              onClick={(e) => {
                                e.stopPropagation();
                                handleStart(c.id);
                              }}
                            >
                              开始
                            </button>
                          ) : null}
                          {c.status === "running" ? (
                            <button
                              type="button"
                              class="btn sm secondary"
                              onClick={(e) => {
                                e.stopPropagation();
                                handlePause(c.id);
                              }}
                            >
                              暂停
                            </button>
                          ) : null}
                          {c.status === "paused" ? (
                            <button
                              type="button"
                              class="btn sm secondary"
                              onClick={(e) => {
                                e.stopPropagation();
                                handleResume(c.id);
                              }}
                            >
                              继续
                            </button>
                          ) : null}
                          <button
                            type="button"
                            class="btn sm danger"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleDestroy(c.id);
                            }}
                          >
                            销毁
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>

          <div class="card" style="margin-top:16px;flex:1;min-height:260px;display:flex;flex-direction:column">
            <h3>
              消息列表
              {selectedConsumer ? ` · ${selectedConsumer.name}` : ""}
            </h3>
            {!selectedId ? (
              <div class="empty">选择一个消费者查看消息</div>
            ) : messages.length === 0 ? (
              <div class="empty">暂无消息</div>
            ) : (
              <div class="cs-message-list">
                {messages.map((m) => (
                  <div key={m.id} class={`cs-message ${m.acked ? "acked" : ""}`}>
                    <div class="cs-message-head">
                      <span class="mono">{formatTime(m.timestamp_ms)}</span>
                      <span class="mono">{formatBytes(m.payload_size)}</span>
                      <span class="mono">{m.exchange || "(默认)"}</span>
                      <span class="mono">{m.routing_key || "-"}</span>
                      {m.redelivered ? <span class="pill warn">重投</span> : null}
                      {m.acked ? (
                        <span class="pill ok">已确认</span>
                      ) : selectedConsumer?.auto_ack ? (
                        <span class="pill idle">自动</span>
                      ) : (
                        <button type="button" class="btn sm primary" onClick={() => handleAck(m.id)}>
                          确认
                        </button>
                      )}
                    </div>
                    <pre class="cs-payload">{m.payload}</pre>
                    {Object.keys(m.headers || {}).length > 0 ? (
                      <pre class="cs-headers">{JSON.stringify(m.headers, null, 2)}</pre>
                    ) : null}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
