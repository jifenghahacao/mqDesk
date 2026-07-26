export function ConnectingOverlay({ show, connectionName }) {
  if (!show) return null;
  return (
    <div class="connecting-overlay show" role="status" aria-live="polite">
      <div class="sk-stack">
        <div class="sk sk-banner" />
        <div class="sk-grid">
          <div class="sk sk-card" />
          <div class="sk sk-card" />
          <div class="sk sk-card" />
          <div class="sk sk-card" />
        </div>
      </div>
      <div class="spinner" />
      <div id="connectingText">正在连接「{connectionName || ""}」…</div>
    </div>
  );
}
