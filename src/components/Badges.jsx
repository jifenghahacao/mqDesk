export function HealthBadge({ status }) {
  const label =
    status === "ok"
      ? "正常"
      : status === "warn"
        ? "堆积预警"
        : status === "danger"
          ? "无人消费"
          : status === "idle"
            ? "空闲"
            : status;
  return (
    <span class={`badge-pill ${status}`}>
      <span class={`dot ${status}`} />
      {label}
    </span>
  );
}

export function StatusPill({ status }) {
  const map = {
    sent: { label: "已发送", cls: "s-sent" },
    consumed: { label: "已被消费", cls: "s-consumed" },
    backlog: { label: "仍堆积", cls: "s-backlog" },
    failed: { label: "消费失败", cls: "s-fail" },
  };
  const info = map[status] || { label: status, cls: "s-sent" };
  return (
    <span class={`status-pill ${info.cls}`}>
      <span class={`dot ${info.cls.replace("s-", "")}`} />
      {info.label}
    </span>
  );
}
