import { minimizeWindow, startDragging, toggleMaximizeWindow } from "../lib/api.js";

export function TitleBar() {
  return (
    <div class="titlebar" onMouseDown={startDragging}>
      <div class="brand">
        <div class="logo">M</div>
        <span>MQDesk</span>
        <span class="sub">RabbitMQ 可视化管控台</span>
      </div>
      <div class="win-controls">
        <button type="button" aria-label="最小化" onClick={minimizeWindow} onMouseDown={(e) => e.stopPropagation()}>
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <rect x="2" y="5.5" width="8" height="1" fill="currentColor" />
          </svg>
        </button>
        <button
          type="button"
          aria-label="最大化/还原"
          onClick={toggleMaximizeWindow}
          onMouseDown={(e) => e.stopPropagation()}
        >
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <rect x="2.5" y="2.5" width="7" height="7" stroke="currentColor" stroke-width="1" fill="none" />
          </svg>
        </button>
        <button
          type="button"
          class="close"
          aria-label="最小化到托盘"
          onClick={minimizeWindow}
          onMouseDown={(e) => e.stopPropagation()}
        >
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M3 3L9 9M9 3L3 9" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" />
          </svg>
        </button>
      </div>
    </div>
  );
}
