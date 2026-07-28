import { useEffect, useState } from "preact/hooks";
import { StatusPill } from "../components/Badges.jsx";
import { ConfirmDialog } from "../components/ConfirmDialog.jsx";
import { SearchableSelect } from "../components/SearchableSelect.jsx";
import { Term } from "../components/Term.jsx";
import {
  extractErrorMessage,
  getMessageTrace,
  getQueueDetail,
  listMessageFeed,
  listQueuesPaginated,
  peekQueueMessages,
  publishMessage,
} from "../lib/api.js";
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
  const [tab, setTab] = useState(() => {
    const t = new URLSearchParams(window.location.search).get("tab");
    return ["send", "feed", "inspect"].includes(t) ? t : "feed";
  });
  const [form, setForm] = useState(EMPTY_FORM);
  const [queues, setQueues] = useState([]);
  const [feed, setFeed] = useState([]);
  const [feedFilter, setFeedFilter] = useState("all");
  const [feedLoading, setFeedLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [error, setError] = useState("");
  const [detailItem, setDetailItem] = useState(null);
  const [detailLoading, setDetailLoading] = useState(false);

  // 队列消息探查
  const [inspectQueue, setInspectQueue] = useState("");
  const [inspectCount, setInspectCount] = useState(20);
  const [inspectLoading, setInspectLoading] = useState(false);
  const [inspectMessages, setInspectMessages] = useState([]);
  const [inspectDetail, setInspectDetail] = useState(null);
  const [inspectError, setInspectError] = useState("");

  async function loadQueues() {
    try {
      // 默认 listQueues 只返回 50 条，这里拉取全部
      const result = await listQueuesPaginated({}, { page: 1, page_size: 5000 });
      const list = result.items || [];
      setQueues(list);
      if (list.length > 0 && !form.targetQueue) {
        setForm((prev) => ({ ...prev, targetQueue: list[0].name }));
      }
      if (list.length > 0 && !inspectQueue) {
        setInspectQueue(list[0].name);
      }
    } catch (e) {
      const msg = extractErrorMessage(e);
      console.warn("加载队列列表失败", e);
      toastFail(`加载队列列表失败：${msg}`);
    }
  }

  async function loadFeed() {
    setFeedLoading(true);
    setError("");
    try {
      // 本地追踪记录（通过 MQDesk 发送的消息）
      const filter = feedFilter === "all" ? null : { status: feedFilter };
      const localList = await listMessageFeed(filter);

      // 队列真实消息：sent 筛选时不混合，其它筛选用相同状态过滤后合并
      let queueItems = [];
      if (feedFilter !== "sent") {
        queueItems = await loadQueueMessagesAsFeed(feedFilter === "all" ? null : feedFilter);
      }

      // 合并并简单按时间倒序（队列消息时间取 peek 时刻，可能排在最近发送的消息之后）
      const merged = [...localList, ...queueItems].sort((a, b) => b.time.localeCompare(a.time));
      setFeed(merged);
    } catch (e) {
      setError(extractErrorMessage(e));
    } finally {
      setFeedLoading(false);
    }
  }

  function inferQueueMessageStatus(q) {
    if ((q.ready || 0) > 0) return "backlog";
    if ((q.unacked || 0) > 0 || (q.consumers || 0) > 0) return "consumed";
    return "backlog";
  }

  async function loadQueueMessagesAsFeed(statusFilter = null) {
    const feedItems = [];
    try {
      // 用分页接口拉取全部队列（默认 listQueues 只返回 50 条）
      const result = await listQueuesPaginated({}, { page: 1, page_size: 5000 });
      const queuesList = result.items || [];
      const withMessages = queuesList
        .filter((q) => (q.total || 0) > 0)
        .sort((a, b) => (b.total || 0) - (a.total || 0))
        .slice(0, 20);

      for (const q of withMessages) {
        const status = inferQueueMessageStatus(q);
        if (statusFilter && status !== statusFilter) continue;

        let peekedCount = 0;
        try {
          const messages = await peekQueueMessages(q.name, 5);
          peekedCount = messages.length;
          messages.forEach((msg, idx) => {
            const payloadPreview =
              typeof msg.payload === "string" && msg.payload.length > 80
                ? `${msg.payload.slice(0, 80)}…`
                : String(msg.payload ?? "");
            feedItems.push({
              trace_id: `peek-${q.name}-${idx}-${Date.now()}`,
              time: new Date().toISOString(),
              direction: "received",
              queue_name: q.name,
              exchange: msg.exchange || null,
              routing_key: msg.routing_key || "",
              status,
              summary: payloadPreview || `消息 · ${msg.routing_key || q.name}`,
              payload_preview: payloadPreview,
              payload_size: msg.payload_size || 0,
              content_type: "application/octet-stream",
            });
          });
        } catch (peekErr) {
          console.warn(`探查队列 ${q.name} 消息失败`, peekErr);
        }

        // peek 失败或返回空时，至少展示队列消息汇总，确保用户能看到这 7300 条的存在
        if (peekedCount === 0) {
          feedItems.push({
            trace_id: `queue-summary-${q.name}-${Date.now()}`,
            time: new Date().toISOString(),
            direction: "received",
            queue_name: q.name,
            exchange: null,
            routing_key: "",
            status,
            summary: `队列消息汇总：共 ${q.total} 条（待消费 ${q.ready || 0} / 处理中 ${q.unacked || 0}）`,
            payload_preview: `该队列包含 ${q.total} 条消息。可在「队列消息探查」中选择该队列查看详情。`,
            payload_size: q.total,
            content_type: "text/plain",
          });
        }
      }
      return feedItems;
    } catch (e) {
      console.warn("从队列加载消息失败", e);
      toastFail(`加载队列消息失败：${extractErrorMessage(e)}`);
      return feedItems;
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
    // 兜底加载的队列消息（trace_id 以 peek- 开头）直接本地展示
    const localItem = feed.find((item) => item.trace_id === traceId);
    if (localItem && String(traceId).startsWith("peek-")) {
      setDetailItem(localItem);
      return;
    }
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

  async function handleInspect() {
    if (!inspectQueue) {
      toastFail("请先选择要探查的队列");
      return;
    }
    setInspectLoading(true);
    setInspectError("");
    try {
      const [messages, detail] = await Promise.all([
        peekQueueMessages(inspectQueue, Math.max(1, Math.min(100, inspectCount))),
        getQueueDetail(inspectQueue),
      ]);
      setInspectMessages(messages);
      setInspectDetail(detail);
    } catch (e) {
      const msg = extractErrorMessage(e);
      setInspectError(msg);
      toastFail(`探查失败：${msg}`);
    } finally {
      setInspectLoading(false);
    }
  }

  function formatHeaders(headers) {
    try {
      return JSON.stringify(headers, null, 2);
    } catch {
      return String(headers);
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
        <button type="button" class={tab === "inspect" ? "on" : ""} data-tab="inspect" onClick={() => setTab("inspect")}>
          队列消息探查
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
              <SearchableSelect
                value={form.targetQueue}
                options={queues.map((q) => q.name)}
                onChange={(value) => updateField("targetQueue", value)}
                placeholder={queues.length === 0 ? "（无可用队列）" : "输入队列名或从下拉选择…"}
                disabled={queues.length === 0}
              />
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
      ) : tab === "feed" ? (
        <div class="card">
          <div class="col-title">
            <h3>消息通知列表（消息命运追踪）</h3>
            <button type="button" class="link" onClick={loadFeed}>
              刷新
            </button>
          </div>

          <div class="seg" id="feedFilters" style="margin-bottom:12px;">
            {FILTER_OPTIONS.map((opt) => {
              const count = feed.filter((item) => opt.value === "all" || item.status === opt.value).length;
              return (
                <button
                  key={opt.value}
                  type="button"
                  class={`chip ${feedFilter === opt.value ? "on" : ""}`}
                  data-f={opt.value}
                  onClick={() => setFeedFilter(opt.value)}
                  title={`${opt.label}：${count} 条`}
                >
                  {opt.label} ({count})
                </button>
              );
            })}
          </div>

          {feedLoading ? (
            <div class="empty">
              <span class="spin" style="width:16px; height:16px; border-width:2px; margin-right:8px;" />
              正在加载队列消息…
            </div>
          ) : error ? (
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
      ) : (
        <div class="card">
          <div class="col-title">
            <h3>队列消息探查（不消费）</h3>
            <span class="muted">查看队列里当前真实消息及在线消费者</span>
          </div>

          {inspectError ? (
            <div role="alert" class="banner danger" style="margin-bottom:12px;">
              <div class="grow">
                <p>{inspectError}</p>
              </div>
            </div>
          ) : null}

          <div class="flex" style="gap:12px; align-items:flex-end; flex-wrap:wrap;">
            <div class="field" style="min-width:220px; flex:1;">
              <label>目标队列</label>
              <SearchableSelect
                value={inspectQueue}
                options={queues.map((q) => q.name)}
                onChange={(value) => setInspectQueue(value)}
                placeholder="请选择队列"
              />
            </div>
            <div class="field" style="width:120px;">
              <label>数量</label>
              <input
                type="number"
                class="input"
                min="1"
                max="100"
                value={inspectCount}
                onInput={(e) => setInspectCount(Number(e.target.value))}
              />
            </div>
            <button
              type="button"
              class="btn primary"
              onClick={handleInspect}
              disabled={inspectLoading || !inspectQueue}
            >
              {inspectLoading ? "探查中…" : "探查"}
            </button>
          </div>

          {inspectDetail ? (
            <div class="stat-grid" style="grid-template-columns: repeat(4, 1fr); margin-top:16px;">
              <div class="stat">
                <div class="label">待消费</div>
                <div class="num">{inspectDetail.summary.ready.toLocaleString()}</div>
              </div>
              <div class="stat">
                <div class="label">处理中</div>
                <div class="num">{inspectDetail.summary.unacked.toLocaleString()}</div>
              </div>
              <div class="stat">
                <div class="label">消费者</div>
                <div class="num">{inspectDetail.summary.consumers.toLocaleString()}</div>
              </div>
              <div class="stat">
                <div class="label">总数</div>
                <div class="num">{inspectDetail.summary.total.toLocaleString()}</div>
              </div>
            </div>
          ) : null}

          {inspectDetail?.consumers?.length > 0 ? (
            <div style="margin-top:16px;">
              <h4>在线消费者</h4>
              <table class="tbl">
                <thead>
                  <tr>
                    <th>标签</th>
                    <th>连接</th>
                    <th>地址</th>
                  </tr>
                </thead>
                <tbody>
                  {inspectDetail.consumers.map((c) => (
                    <tr key={c.name}>
                      <td class="mono">{c.name}</td>
                      <td class="mono">{c.connection_name}</td>
                      <td class="mono">{c.peer_address}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : inspectDetail ? (
            <p class="muted" style="margin-top:12px;">暂无在线消费者。</p>
          ) : null}

          {inspectMessages.length > 0 ? (
            <div style="margin-top:16px;">
              <h4>消息列表（前 {inspectMessages.length} 条）</h4>
              <table class="tbl">
                <thead>
                  <tr>
                    <th>#</th>
                    <th>Payload 预览</th>
                    <th>大小</th>
                    <th>Exchange / RoutingKey</th>
                    <th>Headers</th>
                    <th>是否重发</th>
                  </tr>
                </thead>
                <tbody>
                  {inspectMessages.map((m, i) => (
                    <tr key={m.delivery_tag}>
                      <td>{i + 1}</td>
                      <td class="mono" style="max-width:240px; overflow:hidden; text-overflow:ellipsis;">
                        <pre style="margin:0;">{formatPayload(m.payload)}</pre>
                      </td>
                      <td>{m.payload_size} B</td>
                      <td class="mono">
                        {m.exchange}
                        <br />
                        {m.routing_key}
                      </td>
                      <td>
                        <pre style="margin:0; max-width:160px; overflow:auto;">{formatHeaders(m.headers)}</pre>
                      </td>
                      <td>{m.redelivered ? "是" : "否"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : inspectDetail && !inspectLoading ? (
            <p class="muted" style="margin-top:12px;">该队列当前没有可探查的消息。</p>
          ) : null}
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
