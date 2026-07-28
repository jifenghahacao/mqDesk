const INTERVAL_OPTIONS = [
  { value: 5000, label: "5 秒" },
  { value: 15000, label: "15 秒" },
  { value: 30000, label: "30 秒" },
  { value: 60000, label: "60 秒" },
];

export function RefreshToggle({ enabled, interval, onChange }) {
  return (
    <div class="refresh-toggle">
      <label class="refresh-switch">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => onChange({ enabled: e.target.checked, interval })}
        />
        <span class="refresh-slider" />
        <span class="refresh-label">自动刷新</span>
      </label>
      <select
        class="input refresh-interval"
        value={interval}
        disabled={!enabled}
        onChange={(e) => onChange({ enabled, interval: Number(e.target.value) })}
      >
        {INTERVAL_OPTIONS.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}
