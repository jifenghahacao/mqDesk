export function ConnectionRow({ connection, expanded, onToggle, children }) {
  return (
    <div class={`connection-monitor-row ${expanded ? "expanded" : ""}`}>
      <div class="cm-main" role="button" tabindex="0" onClick={onToggle}>
        <span class="cm-caret">{expanded ? "▼" : "▶"}</span>
        <span class="cm-name mono" title={connection.name}>
          {connection.name}
        </span>
        <span class="cm-address">{connection.peer_address}</span>
        <span class="cm-protocol">{connection.protocol}</span>
        <span class={`cm-state ${connection.state}`}>{connection.state}</span>
        <span class="cm-channels">{connection.channel_count} 信道</span>
        <span class="cm-uptime">已连接 {formatDuration(connection.connected_seconds)}</span>
      </div>
      {expanded ? <div class="cm-channels-wrap">{children}</div> : null}
    </div>
  );
}

function formatDuration(seconds) {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
  return `${Math.floor(seconds / 86400)}d`;
}
