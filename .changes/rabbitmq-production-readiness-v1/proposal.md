# RabbitMQ 生产可用性增强（Phase 1）提案

## 背景与动机

MQDesk 当前已实现队列列表、详情、消息发送、手动消费者、告警、审计等 MVP 能力。但在面向数百队列、万级并发消息的生产场景时，存在以下核心缺口：

1. **运维操作缺失**：无法清空队列（Purge）、无法查看/管理绑定（Bindings）、无法查看连接/信道级诊断信息。
2. **无实时刷新**：所有视图依赖手动点击刷新，无法及时发现堆积与消费中断。
3. **性能瓶颈**：队列列表全量返回、消费者列表存在 N+1 查询、前端表格无虚拟滚动，百级队列即出现卡顿。
4. **缺少生产级防护**：无请求限流、无熔断降级、告警检查全量拉取容易压垮 Management API。

本提案聚焦“让 MQDesk 在中小规模生产环境（≤500 队列、单队列 ≤10 万消息堆积）下可用、可控、可观测”。

## 目标

1. 补齐高频运维操作：队列 Purge、Bindings 管理、Connections/Channels 监控。
2. 实现后台自动刷新 + 前端事件推送，让关键状态在 5 秒内触达用户。
3. 消除已知性能瓶颈：分页/虚拟滚动、聚合查询替代 N+1、前端按需渲染。
4. 引入限流与降级，保护 RabbitMQ Management API 不被 MQDesk 自身压垮。

## 范围（In Scope）

### P0：运维操作补齐

- 队列 Purge（清空队列消息，保留队列元数据）。
- Bindings 列表与删除：展示 Queue/Exchange 绑定关系，支持解绑。
- Connections 列表：展示连接级地址、状态、吞吐量概览。
- Channels 列表：展示信道级 prefetch、unacked、吞吐量。

### P0：实时刷新

- 后端后台任务周期性拉取队列摘要，通过 Tauri Event 推送到前端。
- 总览页、队列列表页订阅刷新事件，自动更新数字与告警。
- 用户可开启/关闭自动刷新，并可设置刷新周期（5s/15s/30s/60s）。

### P1：性能优化

- Management API 查询支持服务端分页参数（page / page_size）。
- 队列列表前端使用虚拟滚动，默认每页 50 条。
- 修复 `list_consumers` 的 N+1 查询：一次性拉取队列速率后做 HashMap 映射。
- `peek_queue_messages` 默认 truncate 从 50000 降至 1024，点击后再加载完整 payload。

### P1：限流与降级

- 对 Management API 调用引入令牌桶限流（按命令维度或全局维度）。
- Management API 连续失败 3 次后，后端返回缓存数据并标记为 stale；前端展示降级提示。
- 告警检查按队列数量动态调整周期，避免全量高频拉取。

## 明确不做（Out of Scope）

受 AGENTS.md 与 ADR 约束，以下内容本次不做：

1. **不替换前端框架**：继续使用 Preact，不引入 React/Vue/Svelte（ADR 003）。
2. **不修改液态玻璃核心 Token**：`src/styles/tokens.css` 核心变量不增删改（ADR 003）。
3. **不绕过 Tauri 命令层**：所有 RabbitMQ 调用仍在 Rust 后端执行，前端不直接访问网络/AMQP（ADR 002）。
4. **不在命令层写业务规则**：业务逻辑仍放在 `mqdesk-core`，命令层只做路由与装配（ADR 001）。
5. **不修改 `legacy/` 目录**：旧项目归档保持冻结（ADR 004）。
6. **不实现 Policies/Parameters/Vhosts/Definitions 的完整 CRUD**：本次只补齐 Bindings 与 Purge，集群级策略管理留到后续阶段。
7. **不实现 Shovel/Federation 插件管理**：超出本次“生产可用性基线”范围。
8. **不实现用户权限与 RBAC**：桌面端单用户场景，继续沿用当前系统用户名作为操作人。
9. **不引入外部时序数据库**：速率历史继续复用 Management API 实时数据，不引入 Prometheus/InfluxDB。
10. **不做消息移动/重发/删除单条消息**：已有 `queue-management-redesign` 变更覆盖或超出本次范围。

## 关键决策

- **实时刷新架构**：后端 tokio 任务轮询 + Tauri `emit_to`/`listen` 事件，前端不主动轮询，降低无效请求。
- **分页策略**：Management API 原生支持 `page` 与 `page_size`，复用原生分页；前端虚拟滚动负责渲染性能。
- **限流位置**：在 `ManagementClient` 内部引入异步令牌桶，保护所有 HTTP 调用；AMQP 发布与消费者保持现状（本次不做连接池）。
- **缓存策略**：后端仅缓存队列摘要（用于降级与刷新），不缓存消息 payload 等敏感/大对象。

## 成功标准

- 500 队列场景下，队列列表首屏加载 ≤ 1s（本地 RabbitMQ， Management API 响应正常）。
- 自动刷新开启后，状态变化在 5s 内反映到前端。
- Management API 连续失败时，前端 3s 内展示降级提示，且 UI 不崩溃。
- 后端测试覆盖率：新增 Rust 单元测试 ≥ 80%（新增模块）。
- 前端 lint/build/test 全部通过。
