# Design：MQDesk 监控与安装体验优化

## 1. 架构总览

所有新增业务逻辑继续下沉到 `crates/core`，Tauri 命令层只做参数传递和错误封装。前端通过 `src/lib/api.js` 调用 Tauri 命令，不直接访问 RabbitMQ。

```
前端 (Preact)
  ├─ views/NodesView.jsx
  ├─ views/ConsumersView.jsx
  ├─ views/ConnectionsView.jsx (改)
  ├─ components/Sidebar.jsx (改)
  └─ lib/api.js (改)

Tauri 命令层 (src-tauri/src/commands/)
  ├─ node.rs (新增)
  ├─ consumer.rs (新增)
  ├─ connection.rs (改)
  └─ lib.rs (注册命令)

crates/core
  ├─ src/rabbit/management.rs (改：新增 list_nodes / list_consumers)
  ├─ src/models.rs (改：新增 Node / Consumer 等模型)
  └─ src/error.rs (不改)

打包
  └─ src-tauri/windows/installer.nsi / nsis-hooks.nsh (改：自定义进度条)
```

## 2. 数据模型与接口

### 2.1 新增 Rust 模型（`src-tauri/crates/core/src/models.rs`）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub name: String,
    pub running: bool,
    pub os_pid: String,
    pub mem_used: u64,
    pub mem_limit: u64,
    pub mem_alarm: bool,
    pub disk_free_alarm: bool,
    pub fd_used: u64,
    pub fd_total: u64,
    pub sockets_used: u64,
    pub proc_used: u64,
    pub proc_total: u64,
    pub uptime: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerInfo {
    pub consumer_tag: String,
    pub queue_name: String,
    pub channel_details: ChannelDetails,
    pub ack_required: bool,
    pub prefetch_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDetails {
    pub name: String,
    pub connection_name: String,
    pub peer_host: String,
    pub peer_port: u16,
}
```

### 2.2 新增 Tauri 命令

| 命令 | 输入 | 输出 | 所在文件 |
|---|---|---|---|
| `list_nodes` | - | `Vec<NodeInfo>` | `src-tauri/src/commands/node.rs` |
| `list_consumers` | `queue_name: Option<String>` | `Vec<ConsumerInfo>` | `src-tauri/src/commands/consumer.rs` |
| `get_connection_status` | `id: String` | `{ id, connected: bool }` | `src-tauri/src/commands/connection.rs` |

### 2.3 RabbitMQ Management API 调用

- 节点：`GET /api/nodes`
- 消费者：`GET /api/consumers/{vhost}`（当 `queue_name` 为 `Some` 时在前端过滤，避免多次 API 调用）

## 3. 各需求技术方案

### 3.1 安装进度条修复

**问题定位**
- Tauri 默认 NSIS 模板使用标准 `ProgressBar` 控件，在 `Section Install` 中通过 `File` 命令更新进度。
- 由于安装包体积大（200MB）且为 LZMA solid 压缩，`File` 解压时进度条更新粒度粗；同时 Windows 原生前进度条在部分 DPI 缩放比例下存在像素取整误差，导致视觉长度与百分比不一致。

**方案**
1. 复制 Tauri 默认 NSIS 模板到 `src-tauri/windows/installer.nsi`。
2. 在 `tauri.conf.json` 中通过 `bundle.windows.nsis.template` 指定该模板。
3. 在 `Section Install` 中把大文件拆分为多次 `SetDetailsPrint` + `SendMessage` 手动更新进度条：
   - 解压主程序前：`SendMessage $ProgressBar ${PBM_SETPOS} 10 0`
   - 解压 `WebView2Loader.dll` 后：`SendMessage $ProgressBar ${PBM_SETPOS} 30 0`
   - WebView2 安装完成后：`SendMessage $ProgressBar ${PBM_SETPOS} 80 0`
   - 注册表/快捷方式完成后：`SendMessage $ProgressBar ${PBM_SETPOS} 100 0`
4. 将百分比文本与进度条位置绑定，统一由同一函数更新，避免 `DetailPrint` 与控件状态不同步。
5. 对进度条控件设置 `PBS_SMOOTH` 样式，减少像素取整造成的视觉误差。

**可测试性**
- 构建后通过 `7z` 或 `lessmsi` 无法直接读取 NSIS 脚本，因此测试方式为：
  - 静默安装并捕获安装日志（`/S /LOG=install.log`），验证日志中出现 `PBM_SETPOS 10/30/80/100` 关键字。
  - 在 100%、125%、150%、200% DPI 虚拟机中截图，用图像工具测量进度条填充比例。

### 3.2 多 MQ 连接状态显示

**方案**
1. 后端新增 `get_connection_status` 命令：遍历 `AppState::active_connection`，比较传入 ID，返回是否匹配当前活跃连接。
2. 前端 `ConnectionsView` 在 `reload()` 后调用 `getActiveConnection()`，把当前活跃连接 ID 存入本地 state。
3. 卡片状态渲染：
   - `connected`：ID == activeId
   - `connecting`：ID == connectingId（新增本地 state）
   - `failed`：ID == lastFailedId（新增本地 state，连接出错时设置）
   - `idle`：其他
4. 左侧 `Sidebar` 当前连接面板增加：
   - 名称
   - 地址：`{scheme}://{host}:{management_port}`
   - vhost 与用户
5. 连接中/失败状态通过 `ConnectingOverlay` 和卡片局部状态同时反馈。

### 3.3 MQ 集群节点监控

**方案**
1. 后端 `management.rs` 新增 `list_nodes()`：
   - 调用 `GET /api/nodes`
   - 解析每个节点的 `name`, `running`, `os_pid`, `mem_used`, `mem_limit`, `mem_alarm`, `disk_free_alarm`, `fd_used`, `fd_total`, `sockets_used`, `proc_used`, `proc_total`, `uptime`
2. 新增 `src-tauri/src/commands/node.rs`，暴露 `list_nodes`。
3. 前端新增 `src/views/NodesView.jsx`：
   - 表格/卡片列表展示节点
   - 点击行进入 `NodeDetailView` 或展开详情
4. 在 `Sidebar` 新增"节点"导航入口。

### 3.4 消费者信息可视化

**方案**
1. 后端 `management.rs` 新增 `list_consumers()`：
   - 调用 `GET /api/consumers/{vhost}`
   - 解析 `consumer_tag`, `queue.name`, `channel_details.name`, `channel_details.connection_name`, `channel_details.peer_host`, `channel_details.peer_port`, `ack_required`, `prefetch_count`
2. 新增 `src-tauri/src/commands/consumer.rs`，暴露 `list_consumers`。
3. 前端新增 `src/views/ConsumersView.jsx`。
4. 在 `QueueDetailView` 增加"查看消费者"按钮，点击后传入 `queue_name` 过滤展示。

## 4. 依赖关系

- `list_nodes` / `list_consumers` 依赖 `AppState::active_connection`，必须先有活跃连接。
- `get_connection_status` 仅依赖 `AppState`，不触发网络请求。
- 前端 `NodesView` / `ConsumersView` 依赖 `activeConnection` 存在；若不存在展示空状态。
- `ConnectionsView` 状态更新依赖 `Sidebar` 中当前连接面板，两者共享 `activeConnection`。

## 5. 禁止修改的文件

| 文件/目录 | 原因 |
|---|---|
| `legacy/` | AGENTS.md 禁止项 |
| `src/styles/tokens.css` | 液态玻璃核心变量禁止修改 |
| `src/main.jsx` | 无需修改，已通过 `app.jsx` 路由 |
| `src-tauri/tauri.conf.json` 中 `productName/bundle/identifier` | 本次不涉及 |
| `src-tauri/crates/core/src/crypto.rs` | 加密方案已稳定，无需改动 |
| `src-tauri/crates/core/src/storage.rs` 数据结构 | 不引入新表/Tree |

## 6. 测试策略

| 层级 | 方式 |
|---|---|
| Rust core | `cargo test`：mock ManagementClient 或使用本地 RabbitMQ 进行集成测试 |
| Tauri 命令 | 通过 `cargo test` 调用 command handler（已注入 AppState） |
| 前端组件 | Vitest + @testing-library/preact：mock `api.js` 返回值 |
| 安装包 | PowerShell 静默安装 + 文件存在性检查 + DPI 截图（人工辅助） |
| E2E | 可选：tauri-driver 或 playwright（不在本次必须范围内） |

## 7. 风险与回退

- **NSIS 自定义模板风险**：若自定义模板与 Tauri 新版本不兼容，可能导致构建失败。回退：删除 `template` 配置，恢复默认模板，进度条问题降级为已知限制。
- **Management API 字段差异**：RabbitMQ 3.x 与 4.x 的 `/api/nodes`、`/api/consumers` 字段可能不同。通过 `#[serde(default)]` 兼容缺失字段。
- **性能风险**：集群节点多时列表渲染慢。前端采用虚拟列表或分页（本次先做简单列表，超过 50 条时再加分页）。
