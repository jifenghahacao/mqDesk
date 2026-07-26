// 二次确认弹窗
export function ConfirmDialog({ open, title, desc, danger, okText, onOk, onCancel }) {
  if (!open) return null;
  return (
    <>
      <div class={`scrim ${open ? "show" : ""}`} onClick={onCancel} />
      <div class={`modal ${open ? "show" : ""}`} role="dialog" aria-modal="true">
        <h3>{title || "确认操作"}</h3>
        <p class="muted" style="font-size:13px;margin-top:8px;" dangerouslySetInnerHTML={{ __html: desc || "" }} />
        {danger ? (
          <div class="warn-note">
            <span class="wn-ic">!</span>
            <span>此操作会真实生效，且不可撤销。请确认目标无误。</span>
          </div>
        ) : null}
        <div class="actions">
          <button type="button" class="btn secondary" onClick={onCancel}>
            取消
          </button>
          <button type="button" class={`btn ${danger ? "danger" : "primary"}`} onClick={onOk}>
            {okText || "确认"}
          </button>
        </div>
      </div>
    </>
  );
}
