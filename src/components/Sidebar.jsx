import { disconnect } from "../lib/api.js";

const NAV_ITEMS = [
  { key: "connections", label: "连接", icon: "🔗" },
  { key: "overview", label: "总览", icon: "📊" },
  { key: "queues", label: "队列", icon: "📋" },
  { key: "nodes", label: "集群节点", icon: "🖥️" },
  { key: "consumers", label: "消费者", icon: "👥" },
  { key: "consumer-studio", label: "消费者工作室", icon: "🧪" },
  { key: "messages", label: "消息", icon: "✉️" },
  { key: "settings", label: "设置", icon: "⚙️" },
];

export function Sidebar({ activeView, activeConnection, onNavigate }) {
  async function handleDisconnect() {
    try {
      await disconnect();
      onNavigate("connections");
    } catch (e) {
      console.warn("断开连接失败：", e);
    }
  }

  return (
    <aside class="sidebar">
      <div class="nav-label">导航</div>
      {NAV_ITEMS.map((item) => (
        <button
          key={item.key}
          type="button"
          class={`nav-item ${activeView === item.key ? "active" : ""}`}
          onClick={() => onNavigate(item.key)}
          data-nav={item.key}
        >
          <span class="ico">{item.icon}</span>
          <span>{item.label}</span>
        </button>
      ))}

      <div class="spacer" />

      {activeConnection ? (
        <div class="conn-chip">
          <span class="dot" />
          <b>{activeConnection.name}</b>
          {activeConnection.host}:{activeConnection.management_port}
          <br />
          vhost: {activeConnection.vhost}
          <br />
          <button type="button" class="btn sm ghost" onClick={handleDisconnect} style="margin-top:8px;width:100%;">
            断开
          </button>
        </div>
      ) : (
        <div class="conn-chip">
          <b>未连接</b>
          请先在「连接」页选择并连接 RabbitMQ。
        </div>
      )}
    </aside>
  );
}
