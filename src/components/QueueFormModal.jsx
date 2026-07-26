import { useState } from "preact/hooks";
import { createQueue, extractErrorMessage, updateQueuePolicy } from "../lib/api.js";
import { toastFail, toastOk } from "../lib/toast.js";

const EMPTY_FORM = {
  name: "",
  vhost: "/",
  queue_type: "classic",
  durable: true,
  auto_delete: false,
  max_length: "",
  message_ttl: "",
  dead_letter_exchange: "",
  dead_letter_routing_key: "",
};

const TYPE_OPTIONS = [
  { value: "classic", label: "Classic" },
  { value: "quorum", label: "Quorum" },
  { value: "stream", label: "Stream" },
];

export function QueueFormModal({ open, queue, onClose, onSaved }) {
  const isEdit = Boolean(queue);
  const [form, setForm] = useState(() =>
    queue
      ? {
          name: queue.summary.name,
          vhost: queue.summary.vhost,
          queue_type: queue.summary.queue_type,
          durable: queue.summary.durable,
          auto_delete: queue.summary.auto_delete,
          max_length: "",
          message_ttl: "",
          dead_letter_exchange: "",
          dead_letter_routing_key: "",
        }
      : EMPTY_FORM,
  );
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState("");

  function update(field, value) {
    setForm((prev) => ({ ...prev, [field]: value }));
  }

  function validate() {
    if (!form.name.trim()) {
      setError("队列名不能为空");
      return false;
    }
    setError("");
    return true;
  }

  async function submit() {
    if (!validate()) return;
    setSubmitting(true);
    try {
      if (isEdit) {
        await updateQueuePolicy({
          name: form.name,
          vhost: form.vhost,
          max_length: form.max_length ? Number(form.max_length) : null,
          message_ttl: form.message_ttl ? Number(form.message_ttl) : null,
          dead_letter_exchange: form.dead_letter_exchange || null,
          dead_letter_routing_key: form.dead_letter_routing_key || null,
        });
        toastOk("队列策略已更新");
      } else {
        const args = {};
        if (form.max_length) args["x-max-length"] = Number(form.max_length);
        if (form.message_ttl) args["x-message-ttl"] = Number(form.message_ttl);
        if (form.dead_letter_exchange) args["x-dead-letter-exchange"] = form.dead_letter_exchange;
        if (form.dead_letter_routing_key) args["x-dead-letter-routing-key"] = form.dead_letter_routing_key;

        await createQueue({
          name: form.name,
          vhost: form.vhost,
          queue_type: form.queue_type,
          durable: form.durable,
          auto_delete: form.auto_delete,
          arguments: args,
        });
        toastOk("队列创建成功");
        setForm(EMPTY_FORM);
      }
      onSaved();
      onClose();
    } catch (e) {
      const msg = extractErrorMessage(e);
      setError(msg);
      toastFail(`${isEdit ? "更新" : "创建"}失败：${msg}`);
    } finally {
      setSubmitting(false);
    }
  }

  if (!open) return null;

  return (
    <div class="scrim show" onClick={onClose}>
      <div class="modal show" onClick={(e) => e.stopPropagation()}>
        <h3>{isEdit ? "编辑队列" : "新建队列"}</h3>

        {error ? (
          <div role="alert" class="banner danger" style="margin-bottom:12px;">
            <div class="grow">
              <p>{error}</p>
            </div>
          </div>
        ) : null}

        <div class="field">
          <label>队列名</label>
          <input
            class="input"
            value={form.name}
            onInput={(e) => update("name", e.target.value)}
            disabled={isEdit}
            placeholder="例如：order.created"
          />
        </div>

        <div class="field">
          <label>vhost</label>
          <input class="input" value={form.vhost} onInput={(e) => update("vhost", e.target.value)} disabled={isEdit} />
        </div>

        <div class="field">
          <label>类型</label>
          <select
            class="input"
            value={form.queue_type}
            onChange={(e) => update("queue_type", e.target.value)}
            disabled={isEdit}
          >
            {TYPE_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>

        <div class="field checkbox-row">
          <label class="checkbox">
            <input
              type="checkbox"
              checked={form.durable}
              onChange={(e) => update("durable", e.target.checked)}
              disabled={isEdit}
            />
            持久化（durable）
          </label>
        </div>

        <div class="field checkbox-row">
          <label class="checkbox">
            <input
              type="checkbox"
              checked={form.auto_delete}
              onChange={(e) => update("auto_delete", e.target.checked)}
              disabled={isEdit}
            />
            自动删除（auto_delete）
          </label>
        </div>

        <div class="field">
          <label>最大长度（可选）</label>
          <input
            class="input"
            type="number"
            value={form.max_length}
            onInput={(e) => update("max_length", e.target.value)}
            placeholder="不限制则留空"
          />
        </div>

        <div class="field">
          <label>消息过期时间 ms（可选）</label>
          <input
            class="input"
            type="number"
            value={form.message_ttl}
            onInput={(e) => update("message_ttl", e.target.value)}
            placeholder="例如 60000"
          />
        </div>

        <div class="field">
          <label>死信交换机（可选）</label>
          <input
            class="input"
            value={form.dead_letter_exchange}
            onInput={(e) => update("dead_letter_exchange", e.target.value)}
            placeholder="DLX 名称"
          />
        </div>

        <div class="field">
          <label>死信路由键（可选）</label>
          <input
            class="input"
            value={form.dead_letter_routing_key}
            onInput={(e) => update("dead_letter_routing_key", e.target.value)}
            placeholder="DLK 名称"
          />
        </div>

        <div class="modal-actions">
          <button type="button" class="btn secondary" onClick={onClose}>
            取消
          </button>
          <button type="button" class="btn primary" onClick={submit} disabled={submitting}>
            {submitting ? "保存中…" : "保存"}
          </button>
        </div>
      </div>
    </div>
  );
}
