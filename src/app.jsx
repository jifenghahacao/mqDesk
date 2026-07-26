import { useEffect, useState } from "preact/hooks";
import { ConnectingOverlay } from "./components/ConnectingOverlay.jsx";
import { Sidebar } from "./components/Sidebar.jsx";
import { TitleBar } from "./components/TitleBar.jsx";
import { ToastContainer } from "./components/Toast.jsx";
import { getActiveConnection, restoreLastActive } from "./lib/api.js";
import { ConnectionsView } from "./views/ConnectionsView.jsx";
import { ConsumerStudio } from "./views/ConsumerStudio.jsx";
import { ConsumersView } from "./views/ConsumersView.jsx";
import { ManualView } from "./views/ManualView.jsx";
import { MessagesView } from "./views/MessagesView.jsx";
import { NodesView } from "./views/NodesView.jsx";
import { OverviewView } from "./views/OverviewView.jsx";
import { QueueDetailView } from "./views/QueueDetailView.jsx";
import { QueuesView } from "./views/QueuesView.jsx";
import { SettingsView } from "./views/SettingsView.jsx";

const DEFAULT_VIEW = "connections";

function getInitialView() {
  const params = new URLSearchParams(window.location.search);
  return params.get("view") || DEFAULT_VIEW;
}

function getInitialQueue() {
  const params = new URLSearchParams(window.location.search);
  return params.get("queue") || null;
}

export function App() {
  const [view, setView] = useState(getInitialView());
  const [activeConnection, setActiveConnection] = useState(null);
  const [activeQueue, setActiveQueue] = useState(getInitialQueue());
  const [connecting, setConnecting] = useState({ show: false, name: "" });

  // 启动时尝试恢复上次活跃连接；否则检查是否已有活跃连接
  // 若 URL 中已指定 view 参数，则保留该视图（用于截图/直达）
  useEffect(() => {
    (async () => {
      try {
        const hasViewParam = new URLSearchParams(window.location.search).has("view");
        const restored = await restoreLastActive();
        if (restored) {
          setActiveConnection(restored);
          if (!hasViewParam) setView("overview");
          return;
        }
        const active = await getActiveConnection();
        if (active) {
          setActiveConnection(active);
          if (!hasViewParam) setView("overview");
        }
      } catch (e) {
        console.warn("恢复活跃连接失败：", e);
      }
    })();
  }, []);

  function handleNavigate(nextView) {
    setView(nextView);
  }

  function handleConnected(connection) {
    setActiveConnection(connection);
    setView("overview");
  }

  function handleOpenQueue(queueName) {
    setActiveQueue(queueName);
    setView("queue-detail");
  }

  function handleConnectStart(name) {
    setConnecting({ show: true, name });
  }

  function handleConnectEnd() {
    setConnecting({ show: false, name: "" });
  }

  function renderView() {
    switch (view) {
      case "connections":
        return (
          <ConnectionsView
            onConnected={handleConnected}
            onConnectStart={handleConnectStart}
            onConnectEnd={handleConnectEnd}
          />
        );
      case "overview":
        return <OverviewView onOpenQueue={handleOpenQueue} onNavigate={handleNavigate} />;
      case "queues":
        return <QueuesView onOpenQueue={handleOpenQueue} />;
      case "nodes":
        return <NodesView />;
      case "consumers":
        return <ConsumersView />;
      case "consumer-studio":
        return <ConsumerStudio />;
      case "queue-detail":
        return <QueueDetailView queueName={activeQueue} onBack={() => setView("queues")} />;
      case "messages":
        return <MessagesView />;
      case "settings":
        return <SettingsView onViewManual={() => setView("manual")} />;
      case "manual":
        return <ManualView onBack={() => setView("settings")} />;
      default:
        return null;
    }
  }

  return (
    <div class="window">
      <div class="halo h1" />
      <div class="halo h2" />
      <div class="halo h3" />

      <TitleBar />

      <div class="body">
        <Sidebar activeView={view} activeConnection={activeConnection} onNavigate={handleNavigate} />

        <main id="mainContent" tabindex="-1">
          {renderView()}
        </main>

        <ConnectingOverlay show={connecting.show} connectionName={connecting.name} />
      </div>

      <ToastContainer />
    </div>
  );
}
