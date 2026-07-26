import { useEffect, useState } from "preact/hooks";
import { listConsumers } from "../lib/api.js";
import { extractErrorMessage } from "../lib/api.js";

function formatDuration(totalSeconds) {
  const days = Math.floor(totalSeconds / 86400);
  const hours = Math.floor((totalSeconds % 86400) / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (days > 0) return `${days}天 ${hours}小时`;
  if (hours > 0) return `${hours}小时 ${minutes}分钟`;
  if (minutes > 0) return `${minutes}分 ${seconds}秒`;
  return `${seconds}秒`;
}

export function ConsumersView() {
  const [consumers, setConsumers] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [search, setSearch] = useState("");

  async function reload() {
    setLoading(true);
    setError("");
    try {
      const list = await listConsumers();
      setConsumers(list);
    } catch (e) {
      setError(extractErrorMessage(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    reload();
    const id = setInterval(reload, 10000);
    return () => clearInterval(id);
  }, []);

  const filtered = consumers.filter(
    (c) =>
      c.consumer_tag.toLowerCase().includes(search.toLowerCase()) ||
      c.queue_name.toLowerCase().includes(search.toLowerCase()) ||
      c.client_address.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <section class="view active" data-view="consumers">
      <header class="page-head">
        <div>
          <h1>消费者</h1>
          <p>查看当前 vhost 下所有消费者的连接信息与消费速率</p>
        </div>
        <div class="actions">
          <input
            class="input"
            style="width:240px;"
            placeholder="搜索标签、队列或地址…"
            value={search}
            onInput={(e) => setSearch(e.target.value)}
          />
          <button type="button" class="btn secondary" onClick={reload}>
            刷新
          </button>
        </div>
      </header>

      {error ? (
        <div role="alert" class="banner danger">
          <div class="grow">
            <h2>加载失败</h2>
            <p>{error}</p>
          </div>
        </div>
      ) : null}

      <div class="card">
        {loading && consumers.length === 0 ? (
          <div class="empty">加载中…</div>
        ) : filtered.length === 0 ? (
          <div class="empty">没有匹配的消费者。</div>
        ) : (
          <table class="tbl">
            <thead>
              <tr>
                <th>消费者标签</th>
                <th>订阅队列</th>
                <th>客户端地址</th>
                <th>连接时长</th>
                <th>消费速率</th>
                <th>Ack</th>
                <th>Prefetch</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((c) => (
                <tr key={`${c.connection_name}-${c.consumer_tag}`}>
                  <td class="mono">{c.consumer_tag}</td>
                  <td class="mono">{c.queue_name}</td>
                  <td class="mono">{c.client_address}</td>
                  <td>{c.connected_seconds > 0 ? formatDuration(c.connected_seconds) : "-"}</td>
                  <td class="mono">{c.message_rate.toFixed(2)} /s</td>
                  <td>{c.ack_required ? "手动" : "自动"}</td>
                  <td class="mono">{c.prefetch_count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </section>
  );
}
