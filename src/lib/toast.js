// Toast 通知系统（轻提示）
// 用 Preact 的 signal 实现，全局单例

import { signal } from "@preact/signals";

const TOAST_TIMEOUT = 3000;

export const toasts = signal([]);

export function showToast(type, message) {
  const id = Math.random().toString(36).slice(2, 10);
  const toast = { id, type, message };
  toasts.value = [...toasts.value, toast];
  setTimeout(() => {
    removeToast(id);
  }, TOAST_TIMEOUT);
}

export function removeToast(id) {
  toasts.value = toasts.value.filter((t) => t.id !== id);
}

export function toastOk(message) {
  showToast("ok", message);
}

export function toastFail(message) {
  showToast("fail", message);
}

export function toastInfo(message) {
  showToast("info", message);
}
