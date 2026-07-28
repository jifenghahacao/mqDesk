import { useEffect, useState } from "preact/hooks";
import { ChannelRow } from "../components/ChannelRow.jsx";
import { ConnectionRow } from "../components/ConnectionRow.jsx";
import { extractErrorMessage, listChannels, listRabbitConnections } from "../lib/api.js";

export function RabbitConnectionsView() {
  const [connections, setConnections] = useState([]);
  const [channels, setChannels] = useState({});
  const [expanded, setExpanded] = useState(new Set());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  async function reload() {
    setLoading(true);
    setError("");
    try {
      const result = await listRabbitConnections();
      setConnections(result.items || []);
    } catch (e) {
      setError(extractErrorMessage(e));
    } finally {
      setLoading(false);
    }
  }

  async function toggleConnection(conn) {
    const next = new Set(expanded);
    if (next.has(conn.name)) {
      next.delete(conn.name);
      setExpanded(next);
      return;
    }
    next.add(conn.name);
    setExpanded(next);

    if (!channels[conn.name]) {
      try {
        const result = await listChannels(conn.name);
        setChannels((prev) => ({ ...prev, [conn.name]: result.items || [] }));
      } catch (e) {
        console.warn("加载信道失败", e);
      }
    }
  }

  useEffect(() => {
    reload();
  }, []);

  if (loading) {
    return (
      <div class="view active">
        <div class="empty">加载中…</div>
      </div>
    );
  }

  if (error) {
    return (
      <div class="view active">
        <div role="alert" class="banner danger">
          <div class="grow">
            <h2>无法加载连接列表</h2>
            <p>{error}</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <section class="view active" data-view="rabbit-connections">
      <header class="page-head">
        <div>
          <h1>连接监控</h1>
          <p>查看当前 RabbitMQ 上的活跃连接与信道</p>
        </div>
        <div class="actions">
          <button type="button" class="btn secondary" onClick={reload}>
            刷新
          </button>
        </div>
      </header>

      <div class="card connection-monitor">
        {connections.length === 0 ? (
          <div class="empty">暂无活跃连接。</div>
        ) : (
          connections.map((conn) => (
            <ConnectionRow
              key={conn.name}
              connection={conn}
              expanded={expanded.has(conn.name)}
              onToggle={() => toggleConnection(conn)}
            >
              {channels[conn.name]?.length > 0 ? (
                <div class="table-wrap">
                  <table class="tbl channel-table">
                    <thead>
                      <tr>
                        <th>信道号</th>
                        <th>消费者</th>
                        <th>Prefetch</th>
                        <th>处理中</th>
                        <th>发布速率</th>
                        <th>投递速率</th>
                        <th>Ack 速率</th>
                      </tr>
                    </thead>
                    <tbody>
                      {channels[conn.name].map((ch) => (
                        <ChannelRow key={ch.name} channel={ch} />
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <div class="empty sm">该连接下暂无信道数据。</div>
              )}
            </ConnectionRow>
          ))
        )}
      </div>
    </section>
  );
}
