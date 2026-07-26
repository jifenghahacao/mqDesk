import { useEffect, useState } from "preact/hooks";
import { StatusPill } from "../components/Badges.jsx";
import { ConfirmDialog } from "../components/ConfirmDialog.jsx";
import { Term } from "../components/Term.jsx";
import { extractErrorMessage, getMessageTrace, listMessageFeed, listQueues, publishMessage } from "../lib/api.js";
import { toastFail, toastInfo, toastOk } from "../lib/toast.js";

const EMPTY_FORM = {
  mode: "direct", // direct | exchange
  targetQueue: "",
  exchange: "",
  routingKey: "",
  payload: "",
  contentType: "application/json",
  deliveryMode: 2,
  mandatory: true,
};

const EXAMPLE = `{
  "userId": 20931,
  "action": "refund",
  "amount": 9900,
  "reason": "用户申请"
}`;

const FILTER_OPTIONS = [
  { value: "all", label: "全部" },
  { value: "sent", label: "已发送" },
  { value: "consumed", label: "已被消费" },
  { value: "backlog", label: "仍堆积" },
  { value: "failed", label: "消费失败" },
];

export function MessagesView() {
  const [tab, setTab] = useState("send"); // send | feed
  const [form, setForm] = useState(EMPTY_FORM);
  const [queues, setQueues] = useState([]);
  const [feed, setFeed] = useState([]);
  const [feedFilter, setFeedFilter] = useState("all");
  const [submitting, setSubmitting] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [error, setError] = useState("");
  const [detailItem, setDetailItem] = useState(null);
  const [detailLoading, setDetailLoading] = useState(false);

  async function loadQueues() {
    try {
      const list = await listQueues();
      setQueues(list);
      if (list.length > 0 && !form.targetQueue) {
        setForm((prev) => ({ ...prev, targetQueue: list[0].name }));
      }
    } catch (e) {
      console.warn("加载队列列表失败", e);
    }
  }

  async function loadFeed() {
    try {
      const filter = feedFilter === "all" ? null : { status: feedFilter };
      const list = await listMessageFeed(filter);
      setFeed(list);
    } catch (e) {
      setError(extractErrorMessage(e));
    }
  }

  useEffect(() => {
    loadQueues();
  }, []);

  useEffect(() => {
    if (tab === "feed") loadFeed();
  }, [tab, feedFilter]);

  function updateField(field, value) {
    setForm((prev) => ({ ...prev, [field]: value }));
  }

  function setMode(mode) {
    setForm((prev) => ({ ...prev, mode }));
  }

  function fillExample() {
    setForm((prev) => ({ ...prev, payload: EXAMPLE }));
    toastInfo("已填入示例消息");
  }

  function validatePayload() {
    if (form.contentType.includes("json")) {
      try {
        JSON.parse(form.payload);
      } catch {
        toastFail("消息不是合法 JSON，已拦截");
        return false;
      }
    }
    if (!form.payload.trim()) {
      toastFail("消息内容不能为空");
      return false;
    }
    return true;
  }

  async function handleSubmit() {
    if (!validatePayload()) return;
    setConfirmOpen(true);
  }

  async function confirmPublish() {
    setConfirmOpen(false);
    setSubmitting(true);
    try {
      const request = {
        target_queue: form.mode === "direct" ? form.targetQueue : null,
        exchange: form.mode === "exchange" ? form.exchange : null,
        routing_key: form.routingKey || form.targetQueue,
        payload: form.payload,
        content_type: form.contentType,
        delivery_mode: form.deliveryMode,
        mandatory: form.mandatory,
      };
      const result = await publishMessage(request);
      if (result.status === "confirmed") {
        toastOk("消息已发送成功");
      } else if (result.status === "returned") {
        toastFail(`消息被退回：${result.reply_text || "路由不可达"}`);
      } else {
        toastFail(`发送失败：${result.error || "未知错误"}`);
      }
      // 切到消息流查看
      setTab("feed");
    } catch (e) {
      toastFail(`发送失败：${extractErrorMessage(e)}`);
    } finally {
      setSubmitting(false);
    }
  }

  const filteredFeed = feed.filter((item) => {
    if (feedFilter === "all") return true;
    return item.status === feedFilter;
  });

  async function openDetail(traceId) {
    setDetailLoading(true);
    try {
      const item = await getMessageTrace(traceId);
      setDetailItem(item);
    } catch (e) {
      toastFail(`加载详情失败：${extractErrorMessage(e)}`);
    } finally {
      setDetailLoading(false);
    }
  }

  function copyPayload() {
    navigator.clipboard.writeText(detailItem.payload_preview).then(
      () => toastOk("已复制 Payload"),
      () => toastFail("复制失败"),
    );
  }

  function formatPayload(value) {
    try {
      return JSON.stringify(JSON.parse(value), null, 2);
    } catch {
      return value;
    }
  }

  return (
    <section class="view active" data-view="messages">
      <header class="page-head">
        <div>
          <h1>消息</h1>
          <p>引导式发送 + 消息命运追踪</p>
        </div>
      </header>

      <div class="seg" style="margin-bottom:16px;">
        <button type="button" class={tab === "send" ? "on" : ""} data-tab="send" onClick={() => setTab("send")}>
          发送消息
        </button>
        <button type="button" class={tab === "feed" ? "on" : ""} data-tab="feed" onClick={() => setTab("feed")}>
          消息通知列表
        </button>
      </div>

      {tab === "send" ? (
        <div class="card">
          <div class="col-title">
            <h3>引导式发送消息</h3>
            <button type="button" class="link" onClick={fillExample}>
              填入示例
            </button>
          </div>

          <div class="seg" style="margin-bottom:12px;">
            <button type="button" class={form.mode === "direct" ? "on" : ""} onClick={() => setMode("direct")}>
              直发队列
            </button>
            <button type="button" class={form.mode === "exchange" ? "on" : ""} onClick={() => setMode("exchange")}>
              经交换机
            </button>
          </div>

          {form.mode === "direct" ? (
            <div class="field">
              <label>
                目标 <Term termKey="queue" label="队列" />
              </label>
              <select
                class="input"
                value={form.targetQueue}
                onChange={(e) => updateField("targetQueue", e.target.value)}
              >
                {queues.length === 0 ? (
                  <option value="">（无可用队列）</option>
                ) : (
                  queues.map((q) => (
                    <option key={q.name} value={q.name}>
                      {q.name}
                    </option>
                  ))
                )}
              </select>
            </div>
          ) : (
            <>
              <div class="field">
                <label>
                  <Term termKey="exchange" label="交换机" />
                </label>
                <input
                  class="input"
                  value={form.exchange}
                  onInput={(e) => updateField("exchange", e.target.value)}
                  placeholder="例如：orders"
                />
              </div>
              <div class="field">
                <label>
                  <Term termKey="routing_key" label="路由键" />
                </label>
                <input
                  class="input"
                  value={form.routingKey}
                  onInput={(e) => updateField("routingKey", e.target.value)}
                  placeholder="例如：order.created"
                />
              </div>
            </>
          )}

          <div class="field">
            <label>
              消息内容（
              <Term termKey="message" label="JSON" />）
            </label>
            <textarea
              class="input"
              rows="8"
              value={form.payload}
              onInput={(e) => updateField("payload", e.target.value)}
              placeholder="输入消息内容…"
            />
          </div>

          <div class="field">
            <label>Content-Type</label>
            <input class="input" value={form.contentType} onInput={(e) => updateField("contentType", e.target.value)} />
          </div>

          <div class="flex" style="gap:12px;">
            <div class="field" style="flex:1">
              <label>持久化</label>
              <select
                class="input"
                value={form.deliveryMode}
                onChange={(e) => updateField("deliveryMode", Number(e.target.value))}
              >
                <option value="2">持久化（推荐）</option>
                <option value="1">非持久化</option>
              </select>
            </div>
            <div class="field" style="flex:1">
              <label>Mandatory 路由校验</label>
              <select
                class="input"
                value={form.mandatory ? "1" : "0"}
                onChange={(e) => updateField("mandatory", e.target.value === "1")}
              >
                <option value="1">开启（找不到队列则退回）</option>
                <option value="0">关闭（消息直接丢弃）</option>
              </select>
            </div>
          </div>

          <div
            class="warn-note"
            style="background:rgba(47,127,242,0.08);border-color:rgba(47,127,242,0.2);color:var(--primary);"
          >
            <span class="wn-ic" style="background:var(--primary);">
              i
            </span>
            <span>
              这条消息将去往：
              {form.mode === "direct"
                ? `队列 ${form.targetQueue || "（未选）"}`
                : `交换机 ${form.exchange || "（未填）"} / 路由键 ${form.routingKey || "（未填）"}`}
              。发送后会在「消息通知列表」中记录其状态。
            </span>
          </div>

          <div class="actions">
            <button type="button" class="btn primary" onClick={handleSubmit} disabled={submitting}>
              {submitting ? (
                <>
                  <span class="spin" style="border-color:rgba(255,255,255,.5);border-top-color:#fff" />
                  发送中
                </>
              ) : (
                "发送消息"
              )}
            </button>
          </div>
        </div>
      ) : (
        <div class="card">
          <div class="col-title">
            <h3>消息通知列表（消息命运追踪）</h3>
            <button type="button" class="link" onClick={loadFeed}>
              刷新
            </button>
          </div>

          <div class="seg" id="feedFilters" style="margin-bottom:12px;">
            {FILTER_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                type="button"
                class={`chip ${feedFilter === opt.value ? "on" : ""}`}
                data-f={opt.value}
                onClick={() => setFeedFilter(opt.value)}
              >
                {opt.label}
              </button>
            ))}
          </div>

          {error ? (
            <div role="alert" class="banner danger">
              <div class="grow">
                <p>{error}</p>
              </div>
            </div>
          ) : filteredFeed.length === 0 ? (
            <div class="empty">该筛选下暂无消息</div>
          ) : (
            <table class="tbl">
              <thead>
                <tr>
                  <th>时间</th>
                  <th>方向</th>
                  <th>队列</th>
                  <th>状态</th>
                  <th>摘要</th>
                </tr>
              </thead>
              <tbody>
                {filteredFeed.map((item) => (
                  <tr key={item.trace_id} class="queue-row" onClick={() => openDetail(item.trace_id)}>
                    <td class="mono">{item.time}</td>
                    <td>{item.direction === "sent" ? "发送" : "接收"}</td>
                    <td class="mono">{item.queue_name}</td>
                    <td>
                      <StatusPill status={item.status} />
                    </td>
                    <td class="muted">{item.summary}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}

      {detailItem ? (
        <div class="scrim show" onClick={() => setDetailItem(null)}>
          <div class="modal show" onClick={(e) => e.stopPropagation()}>
            <h3>消息详情</h3>
            {detailLoading ? (
              <div class="empty">加载中…</div>
            ) : (
              <>
                <div class="field">
                  <label>Trace ID</label>
                  <input class="input" value={detailItem.trace_id} readOnly />
                </div>
                <div class="field">
                  <label>时间</label>
                  <input class="input" value={detailItem.time} readOnly />
                </div>
                <div class="field">
                  <label>
                    <Term termKey="queue" label="队列" />
                  </label>
                  <input class="input" value={detailItem.queue_name} readOnly />
                </div>
                <div class="field">
                  <label>
                    <Term termKey="exchange" label="交换机" />
                  </label>
                  <input class="input" value={detailItem.exchange || "-"} readOnly />
                </div>
                <div class="field">
                  <label>
                    <Term termKey="routing_key" label="路由键" />
                  </label>
                  <input class="input" value={detailItem.routing_key} readOnly />
                </div>
                <div class="field">
                  <label>状态</label>
                  <div>
                    <StatusPill status={detailItem.status} />
                  </div>
                </div>
                <div class="field">
                  <label>Content-Type</label>
                  <input class="input" value={detailItem.content_type} readOnly />
                </div>
                <div class="field">
                  <label>Payload（{detailItem.payload_size} B）</label>
                  <pre class="json-block payload-block">{formatPayload(detailItem.payload_preview)}</pre>
                </div>
                <div class="modal-actions">
                  <button type="button" class="btn secondary" onClick={() => setDetailItem(null)}>
                    关闭
                  </button>
                  <button type="button" class="btn primary" onClick={copyPayload}>
                    复制 Payload
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      ) : null}

      <ConfirmDialog
        open={confirmOpen}
        title="确认发送消息"
        desc={`这条消息将去往：${
          form.mode === "direct"
            ? `队列 <b>${form.targetQueue}</b>`
            : `交换机 <b>${form.exchange}</b> / 路由键 <b>${form.routingKey}</b>`
        }。发送后会在「消息通知列表」中记录其状态。`}
        danger
        okText="确认发送"
        onOk={confirmPublish}
        onCancel={() => setConfirmOpen(false)}
      />
    </section>
  );
}
