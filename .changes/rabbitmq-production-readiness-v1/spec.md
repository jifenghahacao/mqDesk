# RabbitMQ 生产可用性增强（Phase 1）技术规格

> 所有验收标准必须可测试：单元测试、集成测试、E2E 测试或人工 checklist 至少覆盖一种。

---

## 1. 队列 Purge

### 1.1 正常清空

**GIVEN** 用户已连接到一个 RabbitMQ 集群，且当前 vhost 下存在队列 `orders` 并有 1000 条 Ready 消息  
**WHEN** 用户在队列详情页点击「清空队列」并二次确认  
**THEN** 后端调用 `DELETE /api/queues/{vhost}/{name}/contents`，`orders` 的 Ready 与 Total 消息数在 3 秒内变为 0，并写入一条 `purge_queue` 审计日志。

### 1.2 空队列清空

**GIVEN** 队列 `orders` 当前没有任何消息  
**WHEN** 用户执行 Purge  
**THEN** 操作成功返回，消息数保持为 0，不抛出错误。

### 1.3 无权限清空

**GIVEN** 当前连接用户只有 `monitoring` 标签权限  
**WHEN** 用户执行 Purge  
**THEN** 后端返回 403 错误，前端展示「当前账号无权限清空队列」提示，不写入审计日志。

### 1.4 Quorum 队列清空

**GIVEN** 队列 `orders` 类型为 `quorum` 且有消息  
**WHEN** 用户执行 Purge  
**THEN** 调用 Management API 的 Purge 端点；若 RabbitMQ 版本不支持（返回 405），前端提示「该队列类型不支持清空」。

---

## 2. Bindings 管理

### 2.1 展示绑定列表

**GIVEN** 用户打开队列 `orders` 的详情页，且该队列存在 2 条绑定：来自 `ex.orders`（routing_key=`create`）和 `ex.events`（routing_key=`order.*`）  
**WHEN** 详情页切换到「绑定」Tab  
**THEN** 前端展示 2 行绑定记录，每行包含交换机名称、routing_key、绑定参数（arguments）。

### 2.2 删除绑定

**GIVEN** 队列 `orders` 存在到 `ex.orders`、routing_key 为 `create` 的绑定  
**WHEN** 用户点击该绑定行的「解绑」并二次确认  
**THEN** 后端调用 `DELETE /api/bindings/{vhost}/e/ex.orders/q/orders/create`，成功后该绑定从列表消失，并写入 `delete_binding` 审计日志。

### 2.3 删除绑定失败

**GIVEN** 用户尝试删除一个不存在的绑定  
**WHEN** 点击「解绑」  
**THEN** 后端返回 404，前端提示「绑定已不存在或已被删除」，列表自动刷新。

---

## 3. Connections / Channels 监控

### 3.1 连接列表

**GIVEN** 当前 RabbitMQ 集群有 3 个 active AMQP 连接  
**WHEN** 用户导航到「连接」视图  
**THEN** 前端展示 3 行记录，每行包含连接名称、客户端地址、协议、连接时长、channels 数量。

### 3.2 信道列表

**GIVEN** 用户点击某个连接行  
**WHEN** 展开或导航到该连接的「信道」面板  
**THEN** 前端展示该连接下的所有信道，每行包含信道编号、prefetch_count、unacked 数量、consumer_count、publish/deliver 速率。

### 3.3 无连接场景

**GIVEN** 当前集群没有任何连接  
**WHEN** 用户打开「连接」视图  
**THEN** 前端展示空状态「暂无活跃连接」，不报错。

---

## 4. 实时自动刷新

### 4.1 自动刷新开启

**GIVEN** 用户在设置中开启「自动刷新」，周期设置为 5 秒  
**WHEN** 队列 `orders` 的 Ready 数在后台从 100 增加到 500  
**THEN** 5 秒内总览页与队列列表页的 Ready 数字自动更新为 500，无需用户手动刷新。

### 4.2 自动刷新关闭

**GIVEN** 用户关闭「自动刷新」  
**WHEN** 后台队列状态发生变化  
**THEN** 前端数字保持不变，直到用户手动点击刷新按钮。

### 4.3 连接断开后刷新停止

**GIVEN** 自动刷新已开启  
**WHEN** 当前活跃连接断开或切换  
**THEN** 后端后台任务在 1 秒内停止向该窗口发送事件，前端展示「连接已断开」提示。

### 4.4 多窗口事件隔离

**GIVEN** 用户打开了 2 个 MQDesk 窗口  
**WHEN** 自动刷新事件触发  
**THEN** 每个窗口只接收自己的刷新事件，不出现事件串扰（通过 Tauri Event 的 target label 隔离）。

---

## 5. 分页与虚拟滚动

### 5.1 队列列表分页

**GIVEN** 当前 vhost 有 120 个队列，每页大小设置为 50  
**WHEN** 用户打开队列列表  
**THEN** 首次只请求第 1 页（50 条），翻到第 3 页时请求剩余 20 条；总条目数展示为 120。

### 5.2 虚拟滚动渲染

**GIVEN** 队列列表返回 500 条数据  
**WHEN** 用户滚动列表  
**THEN** DOM 中同时存在的行数不超过 30 行（viewport + buffer），滚动帧率 ≥ 30fps。

### 5.3 搜索与分页

**GIVEN** 用户在搜索框输入 `order` 并回车  
**WHEN** 后端按名称过滤后返回 80 条匹配队列  
**THEN** 分页基于过滤后的结果重新计算，展示第 1 页，总条目数为 80。

---

## 6. 消费者列表 N+1 修复

### 6.1 聚合查询

**GIVEN** 当前 vhost 有 100 个队列和 50 个消费者  
**WHEN** 用户打开「消费者」视图  
**THEN** 后端对 Management API 的 HTTP 调用次数 ≤ 5 次（overview/connections/consumers/queues 聚合），不再对每个队列单独调用 `get_queue`。

### 6.2 速率准确性

**GIVEN** 队列 `orders` 当前 deliver_get 速率为 12.5 msg/s  
**WHEN** 消费者视图加载完成  
**THEN** 该队列对应消费者的 `message_rate` 字段显示为 12.5（误差 ±0.1）。

---

## 7. 限流与降级

### 7.1 令牌桶限流

**GIVEN** 限流配置为每秒最多 10 个 Management API 请求  
**WHEN** 前端在 1 秒内发起 15 次刷新请求  
**THEN** 前 10 个请求立即执行，后 5 个请求按 100ms 间隔排队执行，不丢失。

### 7.2 连续失败降级

**GIVEN** Management API 因网络故障连续 3 次返回 5xx 或超时  
**WHEN** 用户打开总览页  
**THEN** 后端返回最近一次缓存的队列摘要，前端在顶部展示黄色横幅「数据可能已过时（stale）」，UI 不崩溃。

### 7.3 恢复自动清除 stale 标记

**GIVEN** 当前处于降级状态  
**WHEN** 下一次 Management API 调用成功  
**THEN** 后端清除 stale 标记，前端隐藏降级横幅。

### 7.4 告警检查降频

**GIVEN** 当前 vhost 有 200 个队列  
**WHEN** 告警后台任务执行  
**THEN** 检查周期为 30 秒（默认）；当队列数 > 100 时，周期自动调整为 60 秒，且单次请求数 ≤ 3 次（使用分页或 overview）。

---

## 8. 边界与异常场景

### 8.1 队列名含特殊字符

**GIVEN** 队列名为 `my.queue/with%special`  
**WHEN** 调用 Purge 或 Bindings 接口  
**THEN** URL 编码正确，操作成功。

### 8.2 vhost 为 `/`

**GIVEN** 当前连接 vhost 为默认 `/`  
**WHEN** 请求 Bindings 或 Purge  
**THEN** URL 中 vhost 被编码为 `%2F`，不导致 404。

### 8.3 后端任务异常

**GIVEN** 后台刷新任务运行中  
**WHEN** 某次轮询抛出异常（如反序列化失败）  
**THEN** 任务记录错误日志并继续下一次轮询，不崩溃整个应用。

### 8.4 大数据消息 preview

**GIVEN** 队列中某条消息 payload 为 1MB 文本  
**WHEN** 用户 peek 消息列表  
**THEN** 列表仅展示前 1024 字符摘要；点击该行后，二次请求才返回完整 payload。

---

## 9. 可测试性要求

| 验收项 | 测试方式 | 最低覆盖 |
|--------|----------|----------|
| Purge API | Rust 单元测试（mock server） | 正常/无权限/空队列 |
| Bindings API | Rust 单元测试 | 列表/删除/404 |
| Connections/Channels | Rust 单元测试 | 解析模型正确性 |
| 自动刷新事件 | 集成测试或 E2E | 事件触发与接收 |
| 分页参数 | Rust 单元测试 | page/page_size 正确传递 |
| 虚拟滚动 | 前端单元测试 | DOM 行数限制 |
| N+1 修复 | Rust 单元测试 | 调用次数断言 |
| 限流 | Rust 单元测试 | 令牌桶行为 |
| 降级 | Rust 单元测试 + 前端测试 | stale 标记与 UI 提示 |
