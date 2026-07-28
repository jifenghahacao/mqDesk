import { useEffect, useState } from "preact/hooks";
import { HealthBadge, StatusPill } from "../components/Badges.jsx";
import { Term } from "../components/Term.jsx";
import { getOverview, listenQueueRefreshed } from "../lib/api.js";
import { extractErrorMessage } from "../lib/api.js";

export function OverviewView({ onOpenQueue, onNavigate }) {
  const [overview, setOverview] = useState(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState("");

  async function reload(silent = false) {
    if (silent) {
      setRefreshing(true);
    } else {
      setLoading(true);
    }
    setError("");
    try {
      const data = await getOverview();
      setOverview(data);
    } catch (e) {
      setError(extractErrorMessage(e));
    } finally {
      if (silent) {
        setRefreshing(false);
      } else {
        setLoading(false);
      }
    }
  }

  useEffect(() => {
    reload(false);
  }, []);

  useEffect(() => {
    let unlisten;
    listenQueueRefreshed(() => {
      reload(true);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

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
            <h2>无法加载总览</h2>
            <p>{error}</p>
          </div>
        </div>
      </div>
    );
  }
  if (!overview) return null;

  const bannerClass =
    overview.health === "ok"
      ? "ok"
      : overview.health === "warn"
        ? "warn"
        : overview.health === "danger"
          ? "danger"
          : "idle";

  return (
    <section class="view active" data-view="overview">
      <header class="page-head">
        <div>
          <h1>总览</h1>
          <p>一眼掌握 RabbitMQ 整体健康度</p>
        </div>
        <div class="actions">
          <button
            type="button"
            class="btn secondary"
            onClick={() => reload(true)}
            disabled={refreshing}
            aria-busy={refreshing}
          >
            {refreshing ? (
              <>
                <span class="spin" style="border-color:rgba(255,255,255,.5);border-top-color:#fff;margin-right:6px;" />
                刷新中
              </>
            ) : (
              "刷新"
            )}
          </button>
        </div>
      </header>

      <div class={`banner ${bannerClass}`}>
        <span class={`status-dot ${bannerClass}`} />
        <div class="grow">
          <h2>{overview.summary}</h2>
          <p>{overview.summary_detail}</p>
        </div>
      </div>

      <div class="stat-grid">
        <button type="button" class="stat" onClick={() => onNavigate("queues")}>
          <div class="label">
            <Term termKey="queue" label="队列总数" />
          </div>
          <div class="num">{overview.stats.queue_count}</div>
          <div class="meta">点击查看列表</div>
        </button>
        <button type="button" class="stat" onClick={() => onNavigate("queues")}>
          <div class="label">
            <Term termKey="exchange" label="交换机总数" />
          </div>
          <div class="num">{overview.stats.exchange_count}</div>
          <div class="meta">所有 vhost 内</div>
        </button>
        <button type="button" class="stat" onClick={() => onNavigate("messages")}>
          <div class="label">
            <Term termKey="message" label="消息总数" />
          </div>
          <div class="num">{overview.stats.total_messages}</div>
          <div class="meta">
            含 <Term termKey="ready" label="待消费" /> + <Term termKey="unacked" label="处理中" />
          </div>
        </button>
        <button
          type="button"
          class={`stat ${overview.stats.alert_count > 0 ? "alert" : ""}`}
          onClick={() => onNavigate("queues")}
        >
          <div class="label">当前告警</div>
          <div class="num">{overview.stats.alert_count}</div>
          <div class="meta">点击查看详情</div>
        </button>
      </div>

      <div class="cols">
        <div class="card">
          <div class="col-title">
            <h3>告警队列</h3>
            <button type="button" class="link" onClick={() => onNavigate("queues")}>
              查看全部 →
            </button>
          </div>
          {overview.alerts.length === 0 ? (
            <div class="empty">没有告警，一切正常。</div>
          ) : (
            <div class="list">
              {overview.alerts.map((alert) => (
                <div
                  key={alert.queue_name}
                  class="row"
                  role="button"
                  tabindex="0"
                  onClick={() => onOpenQueue(alert.queue_name)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      onOpenQueue(alert.queue_name);
                    }
                  }}
                >
                  <span class={`status-dot ${alert.health}`} />
                  <div class="grow">
                    <div class="name mono">{alert.queue_name}</div>
                    <div class="desc">{alert.reason}</div>
                  </div>
                  <span class="muted">待消费 {alert.ready}</span>
                </div>
              ))}
            </div>
          )}
        </div>

        <div class="card">
          <div class="col-title">
            <h3>最近消息流</h3>
            <button type="button" class="link" onClick={() => onNavigate("messages")}>
              查看全部 →
            </button>
          </div>
          {overview.recent_feed.length === 0 ? (
            <div class="empty">还没有消息记录。</div>
          ) : (
            <div class="list">
              {overview.recent_feed.map((item) => (
                <div key={item.trace_id} class="row">
                  <StatusPill status={item.status} />
                  <div class="grow">
                    <div class="name mono">{item.queue_name}</div>
                    <div class="desc">
                      {item.time} · {item.summary}
                    </div>
                  </div>
                  <span class="muted">{item.direction === "sent" ? "发" : "收"}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
