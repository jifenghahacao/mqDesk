import { toasts } from "../lib/toast.js";

export function ToastContainer() {
  if (toasts.value.length === 0) return null;
  return (
    <div class="toast-wrap">
      {toasts.value.map((t) => {
        const icon = t.type === "ok" ? "✓" : t.type === "fail" ? "!" : "i";
        return (
          <div class={`toast ${t.type}`} key={t.id}>
            <span class="ico">{icon}</span>
            <span>{t.message}</span>
          </div>
        );
      })}
    </div>
  );
}
