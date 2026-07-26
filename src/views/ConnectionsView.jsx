import { useEffect, useState } from "preact/hooks";
import { ConfirmDialog } from "../components/ConfirmDialog.jsx";
import { Term } from "../components/Term.jsx";
import {
  connectTo,
  createConnection,
  deleteConnection,
  getConnectionStatus,
  listConnections,
  testConnection,
  updateConnection,
} from "../lib/api.js";
import { extractErrorMessage } from "../lib/api.js";
import { toastFail, toastOk } from "../lib/toast.js";

const EMPTY_FORM = {
  id: null,
  name: "",
  host: "localhost",
  amqp_port: 5672,
  management_port: 15672,
  management_scheme: "http",
  vhost: "/",
  username: "guest",
  password: "",
};

export function ConnectionsView({ onConnected, onConnectStart, onConnectEnd }) {
  const [connections, setConnections] = useState([]);
  const [statuses, setStatuses] = useState({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [editing, setEditing] = useState(null); // null | 'create' | form-data
  const [form, setForm] = useState(EMPTY_FORM);
  const [submitting, setSubmitting] = useState(false);
  const [testing, setTesting] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(null);

  async function loadStatuses(list) {
    const next = {};
    await Promise.all(
      list.map(async (conn) => {
        try {
          const s = await getConnectionStatus(conn.id);
          next[conn.id] = s;
        } catch (e) {
          next[conn.id] = {
            id: conn.id,
            name: conn.name,
            host: conn.host,
            management_port: conn.management_port,
            vhost: conn.vhost,
            username: conn.username,
            is_active: false,
            is_reachable: false,
            cluster_name: null,
            error: extractErrorMessage(e),
          };
        }
      }),
    );
    setStatuses(next);
  }

  async function reload() {
    setLoading(true);
    setError("");
    try {
      const list = await listConnections();
      setConnections(list);
      loadStatuses(list);
    } catch (e) {
      setError(extractErrorMessage(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    reload();
  }, []);

  useEffect(() => {
    if (connections.length === 0) return;
    const id = setInterval(() => loadStatuses(connections), 5000);
    return () => clearInterval(id);
  }, [connections]);

  function openCreate() {
    setForm(EMPTY_FORM);
    setEditing("create");
  }

  function openEdit(conn) {
    setForm({ ...EMPTY_FORM, ...conn, password: "" });
    setEditing("edit");
  }

  function closeForm() {
    setEditing(null);
  }

  function updateField(field, value) {
    setForm((prev) => ({ ...prev, [field]: value }));
  }

  async function handleTest() {
    setTesting(true);
    try {
      const message = await testConnection(form);
      toastOk(message);
    } catch (e) {
      toastFail(`测试失败：${extractErrorMessage(e)}`);
    } finally {
      setTesting(false);
    }
  }

  async function handleSave() {
    setSubmitting(true);
    try {
      let conn;
      if (editing === "create") {
        conn = await createConnection(form);
        // 「保存并连接」：创建后自动连接
        onConnectStart(conn.name);
        try {
          const active = await connectTo(conn.id);
          onConnected(active);
          toastOk(`已连接到「${conn.name}」`);
        } catch (e) {
          toastFail(`连接失败：${extractErrorMessage(e)}`);
        } finally {
          onConnectEnd();
        }
      } else {
        await updateConnection(form.id, form);
        toastOk("连接已更新");
      }
      setEditing(null);
      await reload();
    } catch (e) {
      toastFail(`保存失败：${extractErrorMessage(e)}`);
    } finally {
      setSubmitting(false);
    }
  }

  async function handleConnect(conn) {
    onConnectStart(conn.name);
    try {
      const active = await connectTo(conn.id);
      onConnected(active);
      toastOk(`已连接到「${conn.name}」`);
    } catch (e) {
      toastFail(`连接失败：${extractErrorMessage(e)}`);
    } finally {
      onConnectEnd();
    }
  }

  function handleDelete(conn) {
    setConfirmDelete(conn);
  }

  async function doDelete() {
    if (!confirmDelete) return;
    try {
      await deleteConnection(confirmDelete.id);
      toastOk(`已删除「${confirmDelete.name}」`);
      setConfirmDelete(null);
      await reload();
    } catch (e) {
      toastFail(`删除失败：${extractErrorMessage(e)}`);
    }
  }

  return (
    <section class="view active" data-view="connections">
      <header class="page-head">
        <div>
          <h1>连接管理</h1>
          <p>保存多个 RabbitMQ 连接配置，下次直接打开</p>
        </div>
        <div class="actions">
          <button type="button" class="btn primary" onClick={openCreate}>
            + 新建连接
          </button>
        </div>
      </header>

      {error ? (
        <div role="alert" class="banner danger">
          <div class="grow">
            <h2>加载连接列表失败</h2>
            <p>{error}</p>
          </div>
        </div>
      ) : null}

      {loading ? (
        <div class="empty">加载中…</div>
      ) : connections.length === 0 ? (
        <div class="card">
          <div class="empty">还没有保存的连接。点击右上角「+ 新建连接」开始。</div>
        </div>
      ) : (
        <div class="conn-grid">
          {connections.map((conn) => {
            const status = statuses[conn.id];
            const isActive = status?.is_active ?? false;
            const isReachable = status?.is_reachable ?? false;
            const statusDot = isActive ? "ok" : isReachable ? "ok" : "danger";
            const statusText = isActive ? "当前活跃" : isReachable ? "在线" : status ? "离线" : "检测中…";
            return (
              <div key={conn.id} class="conn-card">
                <div class="name">
                  <span class={`dot ${statusDot}`} />
                  {conn.name}
                </div>
                <div class="meta">
                  {conn.management_scheme}://{conn.host}:{conn.management_port}
                </div>
                <div class="meta">
                  vhost: {conn.vhost} · 用户: {conn.username}
                </div>
                <div class="meta status-line">
                  状态：<span class={`status-text ${statusDot}`}>{statusText}</span>
                  {status?.cluster_name ? ` · 集群：${status.cluster_name}` : null}
                </div>
                <div class="actions">
                  {!isActive ? (
                    <button type="button" class="btn sm primary" onClick={() => handleConnect(conn)}>
                      连接
                    </button>
                  ) : (
                    <span class="badge active">已连接</span>
                  )}
                  <button type="button" class="btn sm secondary" onClick={() => openEdit(conn)}>
                    编辑
                  </button>
                  <button type="button" class="btn sm danger" onClick={() => handleDelete(conn)}>
                    删除
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {editing ? (
        <>
          <div class="scrim show" onClick={closeForm} />
          <div class="modal show" role="dialog" aria-modal="true">
            <h3>{editing === "create" ? "新建连接" : "编辑连接"}</h3>
            <div class="field">
              <label>名称</label>
              <input
                class="input"
                value={form.name}
                onInput={(e) => updateField("name", e.target.value)}
                placeholder="例如：本地开发"
              />
            </div>
            <div class="flex" style="gap:12px;">
              <div class="field" style="flex:2">
                <label>主机地址</label>
                <input
                  class="input"
                  value={form.host}
                  onInput={(e) => updateField("host", e.target.value)}
                  placeholder="localhost 或 IP"
                />
              </div>
              <div class="field" style="flex:1">
                <label>
                  <Term termKey="vhost" label="虚拟空间" />
                </label>
                <input class="input" value={form.vhost} onInput={(e) => updateField("vhost", e.target.value)} />
              </div>
            </div>
            <div class="flex" style="gap:12px;">
              <div class="field" style="flex:1">
                <label>AMQP 端口</label>
                <input
                  class="input"
                  type="number"
                  value={form.amqp_port}
                  onInput={(e) => updateField("amqp_port", Number(e.target.value))}
                />
              </div>
              <div class="field" style="flex:1">
                <label>管理接口端口</label>
                <input
                  class="input"
                  type="number"
                  value={form.management_port}
                  onInput={(e) => updateField("management_port", Number(e.target.value))}
                />
              </div>
              <div class="field" style="flex:1">
                <label>协议</label>
                <select
                  class="input"
                  value={form.management_scheme}
                  onChange={(e) => updateField("management_scheme", e.target.value)}
                >
                  <option value="http">http</option>
                  <option value="https">https</option>
                </select>
              </div>
            </div>
            <div class="flex" style="gap:12px;">
              <div class="field" style="flex:1">
                <label>用户名</label>
                <input class="input" value={form.username} onInput={(e) => updateField("username", e.target.value)} />
              </div>
              <div class="field" style="flex:1">
                <label>密码</label>
                <input
                  class="input"
                  type="password"
                  value={form.password}
                  onInput={(e) => updateField("password", e.target.value)}
                  placeholder={editing === "edit" ? "留空表示不修改" : ""}
                />
              </div>
            </div>
            <div class="actions">
              <button type="button" class="btn secondary" onClick={closeForm}>
                取消
              </button>
              <button type="button" class="btn ghost" onClick={handleTest} disabled={testing}>
                {testing ? "测试中…" : "测试连接"}
              </button>
              <button type="button" class="btn primary" onClick={handleSave} disabled={submitting}>
                {editing === "create" ? "保存并连接" : "保存修改"}
              </button>
            </div>
          </div>
        </>
      ) : null}

      <ConfirmDialog
        open={!!confirmDelete}
        title="删除连接"
        desc={`确认删除「${confirmDelete?.name || ""}」？此操作不可撤销。`}
        danger
        okText="确认删除"
        onOk={doDelete}
        onCancel={() => setConfirmDelete(null)}
      />
    </section>
  );
}
