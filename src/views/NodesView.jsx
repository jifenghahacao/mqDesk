import { useEffect, useState } from "preact/hooks";
import { listNodes } from "../lib/api.js";
import { extractErrorMessage } from "../lib/api.js";

function formatBytes(bytes) {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log10(bytes) / 3);
  const value = bytes / Math.pow(1000, i);
  return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function formatSeconds(totalSeconds) {
  const days = Math.floor(totalSeconds / 86400);
  const hours = Math.floor((totalSeconds % 86400) / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  if (days > 0) return `${days}天 ${hours}小时`;
  if (hours > 0) return `${hours}小时 ${minutes}分钟`;
  return `${minutes}分钟`;
}

function usageClass(percent) {
  if (percent >= 90) return "danger";
  if (percent >= 70) return "warn";
  return "ok";
}

function MetricBar({ label, used, total, percent, unit = "" }) {
  const cls = usageClass(percent);
  return (
    <div class="node-metric">
      <div class="metric-head">
        <span class="metric-label">{label}</span>
        <span class={`metric-value ${cls}`}>{percent.toFixed(1)}%</span>
      </div>
      <div class="progress-track">
        <div class={`progress-fill ${cls}`} style={`width:${Math.min(percent, 100)}%`} />
      </div>
      <div class="metric-meta">
        {unit ? `${used} ${unit} / ${total} ${unit}` : `${formatBytes(used)} / ${formatBytes(total)}`}
      </div>
    </div>
  );
}

function DiskMetric({ label, free, limit, status }) {
  const statusLabel = { ok: "健康", warn: "偏低", danger: "告警" }[status] || "未知";
  return (
    <div class="node-metric">
      <div class="metric-head">
        <span class="metric-label">{label}</span>
        <span class={`metric-value ${status}`}>{statusLabel}</span>
      </div>
      <div class="progress-track">
        <div class={`progress-fill ${status}`} style="width:100%" />
      </div>
      <div class="metric-meta">
        {formatBytes(free)} 可用 / 告警阈值 {formatBytes(limit)}
      </div>
    </div>
  );
}

export function NodesView() {
  const [nodes, setNodes] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  async function reload() {
    setLoading(true);
    setError("");
    try {
      const list = await listNodes();
      setNodes(list);
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

  if (loading && nodes.length === 0) {
    return (
      <section class="view active" data-view="nodes">
        <header class="page-head">
          <div>
            <h1>集群节点</h1>
            <p>查看 RabbitMQ 集群中所有节点的运行状态与资源占用</p>
          </div>
        </header>
        <div class="empty">加载中…</div>
      </section>
    );
  }

  const runningCount = nodes.filter((n) => n.is_running).length;

  return (
    <section class="view active" data-view="nodes">
      <header class="page-head">
        <div>
          <h1>集群节点</h1>
          <p>
            {nodes.length === 0
              ? "查看 RabbitMQ 集群中所有节点的运行状态与资源占用"
              : `${runningCount}/${nodes.length} 个节点运行中`}
          </p>
        </div>
        <div class="actions">
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

      {nodes.length === 0 && !loading ? (
        <div class="empty">当前连接未返回任何节点信息。</div>
      ) : (
        <div class="node-grid">
          {nodes.map((node) => (
            <div key={node.name} class="node-card">
              <div class="node-head">
                <div class="node-title">
                  <span class={`status-dot ${node.is_running ? "ok" : "danger"}`} />
                  <span class="mono">{node.name}</span>
                </div>
                <span class={`badge-pill ${node.is_running ? "ok" : "danger"}`}>
                  {node.is_running ? "运行中" : "离线"}
                </span>
              </div>

              <div class="node-meta">
                <span>类型：{node.node_type || "-"}</span>
                <span>运行时长：{node.is_running ? formatSeconds(node.uptime_seconds) : "-"}</span>
              </div>

              <div class="node-metrics">
                <MetricBar
                  label="内存使用"
                  used={node.mem_used_bytes}
                  total={node.mem_limit_bytes}
                  percent={node.mem_usage_percent}
                />
                <DiskMetric
                  label="磁盘剩余空间"
                  free={node.disk_free_bytes}
                  limit={node.disk_free_limit_bytes}
                  status={node.disk_free_status}
                />
              </div>

              <div class="node-counters">
                <div class="node-counter">
                  <div class="label">文件描述符</div>
                  <div class="num">
                    {node.fd_used} <small>/ {node.fd_total}</small>
                  </div>
                </div>
                <div class="node-counter">
                  <div class="label">Erlang 进程</div>
                  <div class="num">
                    {node.proc_used} <small>/ {node.proc_total}</small>
                  </div>
                </div>
                <div class="node-counter">
                  <div class="label">Socket</div>
                  <div class="num">
                    {node.sockets_used} <small>/ {node.sockets_total}</small>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
