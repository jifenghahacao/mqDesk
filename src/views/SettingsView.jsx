import { useEffect, useState } from "preact/hooks";
import { RefreshToggle } from "../components/RefreshToggle.jsx";
import { Term } from "../components/Term.jsx";
import { getStoredTheme, setStoredTheme } from "../lib/theme.js";
import { setRefreshEnabled, setRefreshInterval } from "../lib/api.js";

const THEME_OPTIONS = [
  { key: "light", label: "浅色" },
  { key: "dark", label: "暗黑" },
  { key: "system", label: "跟随系统" },
];

const REFRESH_KEY = "mqdesk:refresh";

function getStoredRefresh() {
  try {
    const raw = localStorage.getItem(REFRESH_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    // ignore
  }
  return { enabled: true, interval: 5000 };
}

function setStoredRefresh(config) {
  try {
    localStorage.setItem(REFRESH_KEY, JSON.stringify(config));
  } catch {
    // ignore
  }
}

export function SettingsView({ onViewManual }) {
  const [theme, setTheme] = useState(getStoredTheme());
  const [refresh, setRefresh] = useState(getStoredRefresh());

  useEffect(() => {
    setRefreshEnabled(refresh.enabled).catch(() => {});
    setRefreshInterval(refresh.interval).catch(() => {});
  }, []);

  function handleThemeChange(next) {
    setTheme(next);
    setStoredTheme(next);
  }

  function handleRefreshChange(next) {
    setRefresh(next);
    setStoredRefresh(next);
    setRefreshEnabled(next.enabled).catch(() => {});
    setRefreshInterval(next.interval).catch(() => {});
  }

  return (
    <section class="view active" data-view="settings">
      <header class="page-head">
        <div>
          <h1>设置</h1>
          <p>应用偏好与术语表</p>
        </div>
      </header>

      <div class="card">
        <h3>外观</h3>
        <p class="muted" style="margin-top:4px;">
          切换应用主题色。
        </p>
        <div class="seg" style="margin-top:12px;">
          {THEME_OPTIONS.map((opt) => (
            <button
              key={opt.key}
              type="button"
              class={theme === opt.key ? "on" : ""}
              onClick={() => handleThemeChange(opt.key)}
            >
              {opt.label}
            </button>
          ))}
        </div>
      </div>

      <div class="card" style="margin-top:16px;">
        <h3>自动刷新</h3>
        <p class="muted" style="margin-top:4px;">
          控制后台是否周期性拉取队列状态并推送到当前页面。关闭后数字不会自动更新。
        </p>
        <div style="margin-top:12px;">
          <RefreshToggle enabled={refresh.enabled} interval={refresh.interval} onChange={handleRefreshChange} />
        </div>
      </div>

      <div class="card" style="margin-top:16px;">
        <h3>关于 MQDesk</h3>
        <p class="muted" style="margin-top:8px;">
          MQDesk 是一款面向"小白/初级用户"的 RabbitMQ 桌面可视化管控工具。 让不懂 AMQP、看不懂英文管理后台的人，也能在 5
          分钟内连接服务、看懂队列状态、发送并监听消息。
        </p>
        <p class="muted" style="margin-top:8px;">
          版本：v0.1.0（MVP）
        </p>
      </div>

      <div class="card" style="margin-top:16px;">
        <h3>操作手册</h3>
        <p class="muted" style="margin-top:8px;">
          零基础图文版操作手册，包含安装、界面导览、功能详解、常见问题与术语解释。
        </p>
        <button type="button" class="btn primary" style="margin-top:12px;" onClick={onViewManual}>
          📖 查看操作手册
        </button>
      </div>

      <div class="card" style="margin-top:16px;">
        <h3>堆积阈值</h3>
        <p class="muted" style="margin-top:4px;">
          <Term termKey="ready" label="待消费" /> 超过此数视为堆积预警（默认 1000）。 v1
          暂不支持自定义，将在后续版本支持。
        </p>
      </div>

      <div class="card" style="margin-top:16px;">
        <h3>术语表</h3>
        <table class="tbl" style="margin-top:8px;">
          <thead>
            <tr>
              <th>英文术语</th>
              <th>中文显示</th>
              <th>大白话解释</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>Broker</td>
              <td>消息服务节点</td>
              <td class="muted">跑着 RabbitMQ 的那台服务器</td>
            </tr>
            <tr>
              <td>Virtual Host</td>
              <td>虚拟空间（vhost）</td>
              <td class="muted">一块隔离的"房间"，不同项目用不同房间互不干扰</td>
            </tr>
            <tr>
              <td>Exchange</td>
              <td>交换机</td>
              <td class="muted">消息的"分拣中心"，按规则把消息送到对应队列</td>
            </tr>
            <tr>
              <td>Binding</td>
              <td>绑定规则</td>
              <td class="muted">交换机和队列之间的"连线规则"</td>
            </tr>
            <tr>
              <td>Routing Key</td>
              <td>路由键</td>
              <td class="muted">贴在消息上的"地址标签"，决定它去哪个队列</td>
            </tr>
            <tr>
              <td>Queue</td>
              <td>队列</td>
              <td class="muted">存消息的"桶"，消费者从这里取</td>
            </tr>
            <tr>
              <td>Ready</td>
              <td>待消费</td>
              <td class="muted">已经躺在桶里、等着被取的消息数</td>
            </tr>
            <tr>
              <td>Unacked</td>
              <td>处理中</td>
              <td class="muted">已被取走、但还没确认处理完的消息数</td>
            </tr>
            <tr>
              <td>Message</td>
              <td>消息</td>
              <td class="muted">你要传递的那段数据（通常是 JSON）</td>
            </tr>
            <tr>
              <td>Ack</td>
              <td>消费确认</td>
              <td class="muted">消费者说"我处理完了"的回执</td>
            </tr>
            <tr>
              <td>Consumer</td>
              <td>消费者</td>
              <td class="muted">从队列取消息来处理的程序</td>
            </tr>
            <tr>
              <td>Dead Letter</td>
              <td>死信</td>
              <td class="muted">处理失败/过期被扔到"垃圾箱队列"的消息</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  );
}
