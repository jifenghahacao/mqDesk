# Spec：MQDesk 监控与安装体验优化

## 验收标准（GIVEN / WHEN / THEN）

### 1. 安装进度条修复

#### SC-1.1 进度条视觉与百分比一致
- **GIVEN** 用户运行 `MQDesk_0.1.0_x64-setup.exe` 进入安装页
- **WHEN** 安装进度显示为 `65%`
- **THEN** 进度条绿色填充长度等于进度条总长的 65%，误差不超过 2 个百分点；百分比文本与填充长度实时同步更新。

#### SC-1.2 全阶段进度更新
- **GIVEN** 安装程序在 0%-100% 之间运行
- **WHEN** 安装解压 `mqdesk.exe`、`WebView2Loader.dll` 以及安装 WebView2 运行时
- **THEN** 进度条至少出现 5 次可见更新（0%、25%、50%、75%、100%），不会出现长时间停滞在某一百分比。

#### SC-1.3 DPI 缩放兼容性
- **GIVEN** 系统 DPI 缩放为 125%、150% 或 200%
- **WHEN** 用户运行安装程序
- **THEN** 进度条视觉长度与百分比文本保持一致，无偏移、截断或错位。

#### SC-1.4 可测试性
- **GIVEN** 构建后的安装包
- **WHEN** 静默安装并通过脚本截取安装窗口
- **THEN** 可以通过图像分析或 NSIS 日志验证进度条位置与百分比一致。

---

### 2. 多 MQ 连接状态显示

#### SC-2.1 卡片状态标识
- **GIVEN** 已保存 3 个连接配置 A、B、C，当前仅 A 处于已连接状态
- **WHEN** 用户进入"连接管理"视图
- **THEN** A 卡片显示"已连接"状态（绿色圆点 + 文字），B、C 显示"未连接"（灰色圆点 + 文字）。

#### SC-2.2 连接中/失败状态
- **GIVEN** 用户点击 B 卡片触发连接
- **WHEN** 连接请求进行中或返回错误
- **THEN** B 卡片显示"连接中..."（黄色/蓝色旋转标识）或"连接失败"（红色圆点 + 重试按钮）。

#### SC-2.3 左侧当前连接面板增强
- **GIVEN** 当前已连接到 A
- **WHEN** 用户查看左侧导航栏底部
- **THEN** 面板展示 A 的名称、地址 `http://host:15672`、vhost `/`、用户 `guest`，并提供"断开"操作。

#### SC-2.4 状态实时同步
- **GIVEN** 用户在连接管理页断开 A
- **WHEN** 断开成功
- **THEN** A 卡片状态在 1 秒内变为"未连接"，左侧当前连接面板同步消失。

#### SC-2.5 禁止触碰边界
- **GIVEN** 实现多连接状态功能
- **WHEN** 代码审查时检查 AGENTS.md
- **THEN** 未引入 React/Vue/Svelte，未修改液态玻璃核心 CSS 变量。

---

### 3. MQ 集群节点监控

#### SC-3.1 节点列表展示
- **GIVEN** 当前已连接到一个 RabbitMQ 集群节点
- **WHEN** 用户进入"节点"视图
- **THEN** 页面展示集群内所有节点列表，每行包含节点名称、运行状态（running/not running）、Erlang 进程数、FD 使用量、内存/磁盘告警标记。

#### SC-3.2 单节点详情
- **GIVEN** 用户点击某节点行
- **WHEN** 节点详情展开/进入详情页
- **THEN** 展示该节点的 `mem_used` / `mem_limit` / `fd_used` / `fd_total` / `sockets_used` / `proc_used` / `proc_total` 等字段。

#### SC-3.3 集群状态识别
- **GIVEN** 目标 RabbitMQ 是单节点部署
- **WHEN** 用户进入"节点"视图
- **THEN** 列表仅展示一个节点，无错误提示。

#### SC-3.4 离线错误处理
- **GIVEN** 当前连接已断开
- **WHEN** 用户进入"节点"视图
- **THEN** 页面显示"请先连接 RabbitMQ"提示，不发起无意义请求。

#### SC-3.5 API 不直接提供 CPU
- **GIVEN"** 用户查看节点详情
- **WHEN"** 检查可展示字段
- **THEN"** 不展示 CPU 使用率字段（RabbitMQ Management API 的 `/api/nodes` 不直接返回 CPU 占用）。

---

### 4. 消费者信息可视化

#### SC-4.1 消费者列表
- **GIVEN** 当前已连接 RabbitMQ 且存在活跃消费者
- **WHEN** 用户进入"消费者"视图
- **THEN** 列表展示每个消费者的标签、订阅队列、channel 编号、ack 模式、prefetch count、连接主机/IP。

#### SC-4.2 队列详情入口
- **GIVEN** 用户在队列详情页查看某个队列
- **WHEN** 该队列存在消费者
- **THEN** 页面提供"查看消费者"按钮/链接，点击后跳转/弹窗展示仅过滤该队列的消费者。

#### SC-4.3 空状态
- **GIVEN** 当前 vhost 下无任何消费者
- **WHEN** 用户进入"消费者"视图
- **THEN** 页面展示"暂无消费者"空状态，不报错。

#### SC-4.4 数据实时性
- **GIVEN** 用户在消费者视图停留
- **WHEN** 30 秒自动刷新或用户点击刷新按钮
- **THEN** 列表数据在 3 秒内更新，与 `/api/consumers` 返回一致。

#### SC-4.5 连接断开处理
- **GIVEN** 当前无活跃连接
- **WHEN** 用户进入"消费者"视图
- **THEN** 页面提示"请先连接 RabbitMQ"，不发起 API 请求。

---

### 5. 测试与文档

#### SC-5.1 Rust 单元测试
- **GIVEN** 新增 `list_nodes`、`list_consumers` 命令
- **WHEN** 运行 `cd src-tauri && cargo test`
- **THEN** 所有测试通过，新增命令至少各有一条成功路径测试和一条离线异常路径测试。

#### SC-5.2 前端组件测试
- **GIVEN** 新增 NodesView / ConsumersView 组件
- **WHEN** 运行 `npm test`
- **THEN** 至少覆盖：列表渲染、空状态、加载状态、错误状态。

#### SC-5.3 安装包验证
- **GIVEN** 重新构建的 `MQDesk_0.1.0_x64-setup.exe`
- **WHEN** 在干净 Windows 环境静默安装
- **THEN** 安装成功退出，安装目录包含 `mqdesk.exe` 和 `WebView2Loader.dll`。

#### SC-5.4 文档更新
- **GIVEN** 功能开发完成
- **WHEN** 检查 `docs/` 与 `public/manual/`
- **THEN** 用户手册新增"节点监控""消费者查看""多连接状态"章节，并附带界面标注说明。

#### SC-5.5 代码门禁
- **GIVEN** 提交前运行 `python tooling/checks.py guard`
- **WHEN** 检查通过
- **THEN** build / lint / typecheck / test 全部成功，无新增 warning。
