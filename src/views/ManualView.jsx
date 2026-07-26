export function ManualView({ onBack }) {
  return (
    <section class="view active" data-view="manual">
      <header class="page-head">
        <div>
          <h1>操作手册</h1>
          <p>零基础用户操作指南</p>
        </div>
        <div class="actions">
          <button type="button" class="btn secondary" onClick={onBack}>
            ← 返回设置
          </button>
        </div>
      </header>

      <div
        class="card"
        style="padding: 0; overflow: hidden; display: flex; flex-direction: column; height: calc(100vh - 180px); min-height: 480px;"
      >
        <iframe
          src="/manual/index.html"
          title="MQDesk 操作手册"
          style="width: 100%; height: 100%; border: none; display: block;"
        />
      </div>
    </section>
  );
}
