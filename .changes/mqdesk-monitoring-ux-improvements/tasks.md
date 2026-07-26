# Tasks：MQDesk 监控与安装体验优化

> 原则：每个任务只改一个功能点，完成后可独立验证。禁止在人工确认前写任何代码。

---

## 阶段一：安装进度条修复

### Task 1.1 复制并理解 Tauri 默认 NSIS 模板
- **目标**：获得可修改的 NSIS 模板副本。
- **改动文件**：
  - 新增 `src-tauri/windows/installer.nsi`（从 `src-tauri/target/release/nsis/x64/installer.nsi` 复制，去除绝对路径中的本地目录前缀）
- **验收**：
  - `tauri.conf.json` 中可配置 `bundle.windows.nsis.template` 指向该文件。
  - `npm run tauri:build:win` 仍能成功构建。
- **禁止**：不要修改 NSIS 脚本中的 `File` 目标路径生成逻辑以外的部分。

### Task 1.2 在 NSIS 模板中插入手动进度控制
- **目标**：让进度条百分比与视觉长度精确对应。
- **改动文件**：
  - 改 `src-tauri/windows/installer.nsi`
- **实现要点**：
  - 在 `Section Install` 中找到 `ProgressBar` 控件句柄。
  - 在解压 `mqdesk.exe` 前后、`WebView2Loader.dll` 前后、WebView2 安装前后、注册表写入后，分别调用 `SendMessage $ProgressBar ${PBM_SETPOS} <pos> 0`。
  - 确保 `DetailPrint` 中的百分比文本与进度条位置一致。
- **验收**：
  - 静默安装日志 `/LOG=install.log` 中出现 `PBM_SETPOS` 关键字。
  - 在 100% 缩放下截图，65% 时进度条填充长度 ≈ 总长度 × 0.65。

### Task 1.3 设置进度条平滑样式并验证 DPI
- **目标**：消除 DPI 缩放导致的像素取整误差。
- **改动文件**：
  - 改 `src-tauri/windows/installer.nsi`
- **实现要点**：
  - 对进度条控件添加 `PBS_SMOOTH` 样式位。
- **验收**：
  - 在 125%、150%、200% DPI 设置下截图，进度条无错位。

---

## 阶段二：多 MQ 连接状态显示优化

### Task 2.1 后端新增 `get_connection_status` 命令
- **目标**：让前端能查询某个配置是否为当前活跃连接。
- **改动文件**：
  - 改 `src-tauri/src/commands/connection.rs`
  - 改 `src-tauri/src/lib.rs`（注册命令）
- **实现要点**：
  - 命令签名：`get_connection_status(state: State<'_, Arc<AppState>>, id: String) -> AppResult<bool>`
  - 比较 `state.active_connection` 中的 `connection.id` 与传入 `id`。
- **验收**：
  - `cargo test` 新增测试：当前无活跃连接时返回 false；设置活跃后返回 true。

### Task 2.2 前端 API 封装新增 `getConnectionStatus`
- **目标**：前端可调用新命令。
- **改动文件**：
  - 改 `src/lib/api.js`
- **实现要点**：
  - 添加 `export async function getConnectionStatus(id) { return invoke("get_connection_status", { id }); }`
- **验收**：
  - `npm test` 中 mock 该函数，确保其他视图不因此报错。

### Task 2.3 `ConnectionsView` 卡片状态渲染
- **目标**：根据连接状态显示不同视觉标识。
- **改动文件**：
  - 改 `src/views/ConnectionsView.jsx`
- **实现要点**：
  - 新增本地 state：`activeId`、`connectingId`、`lastFailedId`。
  - `reload()` 后调用 `getActiveConnection()` 更新 `activeId`。
  - `handleConnect` 时设置 `connectingId`，成功/失败时更新。
  - 卡片渲染：`connected` 绿色圆点 + 文字；`connecting` 蓝色/黄色旋转标识；`failed` 红色圆点 + 重试按钮；`idle` 灰色圆点。
- **验收**：
  - Vitest 测试覆盖 4 种状态渲染。

### Task 2.4 `Sidebar` 当前连接面板增强
- **目标**：左下角展示更完整的当前连接信息。
- **改动文件**：
  - 改 `src/components/Sidebar.jsx`
- **实现要点**：
  - 从 `activeConnection` 中读取 `name`、`management_scheme`、`host`、`management_port`、`vhost`、`username`。
  - 展示格式：`名称`、`地址`、`vhost / 用户`。
- **验收**：
  - 组件测试：传入不同 activeConnection，验证文本渲染正确。

---

## 阶段三：MQ 集群节点监控

### Task 3.1 `crates/core` 新增节点模型与 API
- **目标**：后端能拉取并解析集群节点数据。
- **改动文件**：
  - 改 `src-tauri/crates/core/src/models.rs`（新增 `NodeInfo`）
  - 改 `src-tauri/crates/core/src/rabbit/management.rs`（新增 `list_nodes`）
- **实现要点**：
  - 调用 `GET /api/nodes`。
  - 字段加 `#[serde(default)]` 兼容不同 RabbitMQ 版本。
- **验收**：
  - `cargo test` 对 mock 响应解析通过。

### Task 3.2 Tauri 命令暴露 `list_nodes`
- **目标**：前端可调用。
- **改动文件**：
  - 新增 `src-tauri/src/commands/node.rs`
  - 改 `src-tauri/src/lib.rs`（注册命令）
- **实现要点**：
  - 命令层仅调用 `state.rabbit_management().list_nodes().await`。
  - 未连接时返回 `AppError::NotConnected`。
- **验收**：
  - `cargo test` 验证未连接时返回错误，有活跃连接时返回列表。

### Task 3.3 前端新增 `NodesView`
- **目标**：展示节点列表与详情。
- **改动文件**：
  - 新增 `src/views/NodesView.jsx`
  - 改 `src/app.jsx`（添加路由）
  - 改 `src/components/Sidebar.jsx`（添加导航入口）
  - 改 `src/lib/api.js`（添加 `listNodes`）
- **实现要点**：
  - 表格/卡片列表展示：`name`、`running`、`proc_used/total`、`fd_used/total`、`mem_alarm`、`disk_free_alarm`。
  - 点击行展开详情，显示 `mem_used/limit`、`sockets_used`、`uptime`。
  - 无连接时展示空状态。
- **验收**：
  - Vitest 覆盖列表、空状态、错误状态。

---

## 阶段四：消费者信息可视化

### Task 4.1 `crates/core` 新增消费者模型与 API
- **目标**：后端能拉取并解析消费者数据。
- **改动文件**：
  - 改 `src-tauri/crates/core/src/models.rs`（新增 `ConsumerInfo`、`ChannelDetails`）
  - 改 `src-tauri/crates/core/src/rabbit/management.rs`（新增 `list_consumers`）
- **实现要点**：
  - 调用 `GET /api/consumers/{vhost}`。
  - `channel_details` 可能为 null，使用 `#[serde(default)]`。
- **验收**：
  - `cargo test` 对 mock 响应解析通过。

### Task 4.2 Tauri 命令暴露 `list_consumers`
- **目标**：前端可调用。
- **改动文件**：
  - 新增 `src-tauri/src/commands/consumer.rs`
  - 改 `src-tauri/src/lib.rs`（注册命令）
- **实现要点**：
  - 命令层调用 `state.rabbit_management().list_consumers().await`。
  - 返回完整列表，由前端按 `queue_name` 过滤。
- **验收**：
  - `cargo test` 验证未连接/已连接两种场景。

### Task 4.3 前端新增 `ConsumersView`
- **目标**：展示消费者列表。
- **改动文件**：
  - 新增 `src/views/ConsumersView.jsx`
  - 改 `src/app.jsx`（添加路由）
  - 改 `src/components/Sidebar.jsx`（添加导航入口）
  - 改 `src/lib/api.js`（添加 `listConsumers`）
- **实现要点**：
  - 列表展示：`consumer_tag`、`queue_name`、`channel_details.name`、`peer_host:peer_port`、`ack_required`、`prefetch_count`。
  - 支持 30 秒自动刷新 + 手动刷新按钮。
  - 无连接/无消费者时展示对应空状态。
- **验收**：
  - Vitest 覆盖列表、空状态、刷新逻辑。

### Task 4.4 队列详情页增加消费者入口
- **目标**：从队列详情直接查看该队列的消费者。
- **改动文件**：
  - 改 `src/views/QueueDetailView.jsx`
- **实现要点**：
  - 在队列详情页增加"查看消费者"按钮。
  - 点击后跳转到 `ConsumersView`，并通过 query state / app state 传入 `queue_name`。
  - `ConsumersView` 接收过滤条件，仅展示匹配消费者。
- **验收**：
  - 组件测试：传入过滤条件后列表项数量正确。

---

## 阶段五：测试与文档

### Task 5.1 Rust 单元与集成测试
- **目标**：验证后端新增命令。
- **改动文件**：
  - 新增/改 `src-tauri/crates/core/src/*_test.rs` 或 `tests/*.rs`
  - 改 `src-tauri/src/commands/*.rs`（如有必要添加 `#[cfg(test)]`）
- **验收**：
  - `cd src-tauri && cargo test` 全部通过。

### Task 5.2 前端组件测试
- **目标**：验证 UI 行为。
- **改动文件**：
  - 新增 `src/tests/NodesView.test.jsx`
  - 新增 `src/tests/ConsumersView.test.jsx`
  - 新增/改 `src/tests/ConnectionsView.test.jsx`
- **验收**：
  - `npm test` 全部通过。

### Task 5.3 安装包验证
- **目标**：确保安装包正常、进度条修复生效。
- **执行方式**：
  - 运行 `npm run tauri:build:win`。
  - 在测试机/虚拟机静默安装，检查安装目录文件。
  - 截图验证进度条 0/25/50/75/100 各阶段视觉一致性。
- **验收**：
  - 安装成功，`mqdesk.exe` 与 `WebView2Loader.dll` 均存在。
  - 进度条在各阶段无明显视觉偏差。

### Task 5.4 用户手册与测试报告
- **目标**：输出可交付文档。
- **改动文件**：
  - 新增/改 `public/manual/*.md` 或 `docs/manual/*.md`
  - 新增 `.changes/mqdesk-monitoring-ux-improvements/test-report.md`
- **内容**：
  - 手册新增"多连接状态""节点监控""消费者查看"章节。
  - 测试报告记录：测试环境、用例、结果、未解决问题。
- **验收**：
  - 文档通过 `python tooling/checks.py lint` 的 markdown/basic 检查。

### Task 5.5 全量门禁
- **目标**：保证代码质量。
- **执行方式**：
  - 运行 `python tooling/checks.py guard`。
- **验收**：
  - build / lint / typecheck / test 全部成功。
