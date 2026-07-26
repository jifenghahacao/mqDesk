# 队列管理页重构开发任务

## Phase 1：核心重构

### 1.1 后端数据模型与 API
- [ ] 扩展 `QueueSummary` 增加 queue_type / ready / unacked / total 等字段
- [ ] 新增 `QueueDetail`、`QueueMessage`、`QueueAlertRule`、`QueueAlert`、`QueueAuditLog` 模型
- [ ] 实现 `list_queues(filter)` 带筛选
- [ ] 实现 `get_queue_detail(name, vhost)` 含配置和连接信息
- [ ] 实现 `peek_queue_messages(name, vhost, count)`

### 1.2 队列列表页重构
- [ ] 重构 `QueuesView.jsx` 为表格视图
- [ ] 添加搜索 + 类型/状态/vhost 筛选栏
- [ ] 集成 `StatusPill` 展示健康状态
- [ ] 表格行点击打开详情抽屉

### 1.3 详情抽屉概览页
- [ ] 新建 `QueueDetailDrawer.jsx`
- [ ] 实现 Tab 导航
- [ ] 概览页：展示配置参数卡片
- [ ] 概览页：展示上下游连接列表

## Phase 2：消息与操作

### 2.1 消息管理
- [ ] 实现 `republish_message` / `move_message` / `delete_message` 命令
- [ ] 消息列表页：展示 delivery_tag / 时间 / 大小 / 状态
- [ ] 消息详情弹窗：JSON 格式化 + 复制
- [ ] 消息操作：重发 / 移动 / 删除

### 2.2 队列操作
- [ ] 实现 `create_queue` / `update_queue_policy` / `delete_queue` / `pause_queue` / `resume_queue`
- [ ] 新建 `QueueFormModal.jsx`
- [ ] 新建队列表单（基础 + 高级参数）
- [ ] 编辑队列（只展示可改参数）
- [ ] 删除队列二次确认 + 备份选项

## Phase 3：告警与审计

### 3.1 告警
- [ ] 新增告警规则和告警记录存储
- [ ] 实现规则 CRUD 命令
- [ ] 后台任务每 30 秒检查阈值
- [ ] 应用内 Toast + 侧边栏红点通知
- [ ] 告警历史页面

### 3.2 审计
- [ ] 使用 sled 存储审计日志
- [ ] 在所有队列/消息操作后写审计记录
- [ ] 审计日志页面 + 导出 CSV/JSON

## Phase 4：性能分析

### 4.1 趋势图
- [ ] 新增 `SimpleLineChart.jsx` SVG 组件
- [ ] 在概览页展示流入/流出速率趋势（小时/天/周）

### 4.2 诊断报告
- [ ] 实现 `get_queue_performance_report`
- [ ] 新增 `QueuePerformancePanel.jsx`
- [ ] 展示堆积原因、队列类型建议、TTL/死信建议

## Phase 5：联调与门禁

- [ ] 前端 `npm run lint` 通过
- [ ] 前端 `npm run build` 通过
- [ ] 前端 `npm test` 通过
- [ ] 后端 `cargo check` 通过
- [ ] 后端 `cargo test -p mqdesk-core` 通过
- [ ] 手动验证：列表筛选、详情抽屉、消息操作、队列创建/删除、告警触发
