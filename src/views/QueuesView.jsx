import { useEffect, useMemo, useState } from "preact/hooks";
import { HealthBadge } from "../components/Badges.jsx";
import { QueueFormModal } from "../components/QueueFormModal.jsx";
import { Term } from "../components/Term.jsx";
import { VirtualTable } from "../components/VirtualTable.jsx";
import { extractErrorMessage, listQueuesPaginated, listenQueueRefreshed } from "../lib/api.js";

const TYPE_OPTIONS = [
  { value: "all", label: "全部类型" },
  { value: "classic", label: "Classic" },
  { value: "quorum", label: "Quorum" },
  { value: "stream", label: "Stream" },
];

const HEALTH_OPTIONS = [
  { value: "all", label: "全部状态" },
  { value: "ok", label: "正常" },
  { value: "warn", label: "堆积预警" },
  { value: "danger", label: "无人消费" },
  { value: "idle", label: "空闲" },
];

const PAGE_SIZE_OPTIONS = [20, 50, 100, 200];

export function QueuesView({ onOpenQueue }) {
  const [queues, setQueues] = useState([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(50);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [search, setSearch] = useState("");
  const [queueType, setQueueType] = useState("all");
  const [health, setHealth] = useState("all");
  const [formOpen, setFormOpen] = useState(false);

  async function reload(nextPage = page, nextPageSize = pageSize) {
    setLoading(true);
    setError("");
    try {
      const filter = {
        search: search.trim(),
        queue_type: queueType,
      };
      const pagination = { page: nextPage, page_size: nextPageSize };
      const result = await listQueuesPaginated(filter, pagination);
      setQueues(result.items);
      setTotal(result.total);
      setPage(result.page);
      setPageSize(result.page_size);
    } catch (e) {
      setError(extractErrorMessage(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    reload(1, pageSize);
  }, [queueType, health]);

  useEffect(() => {
    let unlisten;
    listenQueueRefreshed(() => {
      reload(page, pageSize);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, [page, pageSize, search, queueType, health]);

  const filteredQueues = useMemo(() => {
    if (health === "all") return queues;
    return queues.filter((q) => q.health === health);
  }, [queues, health]);

  const totals = useMemo(() => {
    const ok = filteredQueues.filter((q) => q.health === "ok").length;
    const warn = filteredQueues.filter((q) => q.health === "warn").length;
    const danger = filteredQueues.filter((q) => q.health === "danger").length;
    const idle = filteredQueues.filter((q) => q.health === "idle").length;
    return { total: filteredQueues.length, ok, warn, danger, idle };
  }, [filteredQueues]);

  return (
    <section class="view active" data-view="queues">
      <header class="page-head">
        <div>
          <h1>队列管理</h1>
          <p>
            查看、筛选并管理所有 <Term termKey="queue" label="队列" />
          </p>
        </div>
        <div class="actions">
          <button type="button" class="btn primary" onClick={() => setFormOpen(true)}>
            + 新建队列
          </button>
        </div>
      </header>

      <div class="stat-grid queue-summary" style="grid-template-columns: repeat(5, 1fr);">
        <div class="stat">
          <div class="label">总队列数</div>
          <div class="num">{totals.total}</div>
        </div>
        <div class="stat ok">
          <div class="label">正常</div>
          <div class="num">{totals.ok}</div>
        </div>
        <div class="stat warn">
          <div class="label">堆积预警</div>
          <div class="num">{totals.warn}</div>
        </div>
        <div class="stat danger">
          <div class="label">无人消费</div>
          <div class="num">{totals.danger}</div>
        </div>
        <div class="stat idle">
          <div class="label">空闲</div>
          <div class="num">{totals.idle}</div>
        </div>
      </div>

      <div class="card queue-toolbar">
        <div class="filter-row">
          <input
            class="input search-input"
            placeholder="搜索队列名…"
            value={search}
            onInput={(e) => setSearch(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && reload()}
          />
          <select class="input filter-select" value={queueType} onChange={(e) => setQueueType(e.target.value)}>
            {TYPE_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
          <select class="input filter-select" value={health} onChange={(e) => setHealth(e.target.value)}>
            {HEALTH_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
          <button type="button" class="btn secondary" onClick={reload}>
            刷新
          </button>
        </div>
      </div>

      {error ? (
        <div role="alert" class="banner danger" style="margin-top:16px;">
          <div class="grow">
            <h2>加载失败</h2>
            <p>{error}</p>
          </div>
        </div>
      ) : null}

      <div class="card" style="margin-top:16px;">
        {loading ? (
          <div class="empty">加载中…</div>
        ) : filteredQueues.length === 0 ? (
          <div class="empty">没有匹配的队列。</div>
        ) : (
          <div class="table-wrap">
            <VirtualTable
              data={filteredQueues}
              columns={10}
              rowHeight={48}
              buffer={8}
              tableClass="queue-table"
              renderHeader={() => (
                <thead>
                  <tr>
                    <th>队列名</th>
                    <th>类型</th>
                    <th>
                      <Term termKey="ready" label="待消费" />
                    </th>
                    <th>
                      <Term termKey="unacked" label="处理中" />
                    </th>
                    <th>总数</th>
                    <th>
                      <Term termKey="consumer" label="消费者" />
                    </th>
                    <th>流入速率</th>
                    <th>流出速率</th>
                    <th>健康度</th>
                    <th></th>
                  </tr>
                </thead>
              )}
              renderRow={(q) => (
                <tr key={q.name} class="queue-row" onClick={() => onOpenQueue(q.name)}>
                  <td class="mono queue-name">{q.name}</td>
                  <td>
                    <span class="type-pill">{q.queue_type}</span>
                  </td>
                  <td class="mono">{q.ready.toLocaleString()}</td>
                  <td class="mono">{q.unacked.toLocaleString()}</td>
                  <td class="mono">{q.total.toLocaleString()}</td>
                  <td class="mono">{q.consumers}</td>
                  <td class="mono">{q.incoming_rate.toFixed(1)}</td>
                  <td class="mono">{q.outgoing_rate.toFixed(1)}</td>
                  <td>
                    <HealthBadge status={q.health} />
                  </td>
                  <td>
                    <button
                      type="button"
                      class="btn sm ghost"
                      onClick={(e) => {
                        e.stopPropagation();
                        onOpenQueue(q.name);
                      }}
                    >
                      查看
                    </button>
                  </td>
                </tr>
              )}
            />
          </div>
        )}
      </div>

      <div class="pagination-bar" style="margin-top:16px;">
        <div class="pagination-info">
          共 {total.toLocaleString()} 条 · 第 {page} 页 · 每页
          <select
            class="input page-size-select"
            value={pageSize}
            onChange={(e) => reload(1, Number(e.target.value))}
            disabled={loading}
          >
            {PAGE_SIZE_OPTIONS.map((size) => (
              <option key={size} value={size}>
                {size}
              </option>
            ))}
          </select>
          条
        </div>
        <div class="pagination-actions">
          <button
            type="button"
            class="btn secondary sm"
            disabled={page <= 1 || loading}
            onClick={() => reload(page - 1, pageSize)}
          >
            上一页
          </button>
          <button
            type="button"
            class="btn secondary sm"
            disabled={page * pageSize >= total || loading}
            onClick={() => reload(page + 1, pageSize)}
          >
            下一页
          </button>
        </div>
      </div>

      <QueueFormModal open={formOpen} queue={null} onClose={() => setFormOpen(false)} onSaved={() => reload(page, pageSize)} />
    </section>
  );
}
