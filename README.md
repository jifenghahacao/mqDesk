# MQDesk

> RabbitMQ 可视化管控台（小白友好的桌面工具）

MQDesk 是一款面向"小白/初级用户"的 RabbitMQ 桌面可视化管控工具，让不懂 AMQP、看不懂英文管理后台的人，也能在 5 分钟内连接服务、看懂队列状态、发送并监听消息。

## 技术栈

- **桌面壳**：Tauri 2.x（Rust + WebView2）→ 安装包 ~10MB，支持 Windows x64 + ARM64
- **前端**：Preact + Vite + Vitest（液态玻璃视觉，原生复用原型 CSS）
- **后端**：Rust + tokio + reqwest（Management HTTP API）+ lapin（AMQP publisher）
- **本地存储**：sled（连接配置、消息流）+ keyring（密码加密，Windows Credential Manager）

## 目录结构

```
.
├── src-tauri/                    # Rust 桌面壳
│   ├── src/
│   │   ├── main.rs               # 入口
│   │   ├── lib.rs                # 库入口（注册 Tauri 命令）
│   │   ├── commands/             # Tauri 命令（前端可调用）
│   │   │   ├── connection.rs     # R1 连接管理
│   │   │   ├── overview.rs       # R2 总览
│   │   │   ├── queue.rs          # R3/R4 队列列表/详情/抓取预览
│   │   │   └── message.rs        # R5/R7 发送 + 消息流
│   │   ├── rabbit/               # RabbitMQ 客户端
│   │   │   ├── management.rs     # Management HTTP API (port 15672)
│   │   │   └── publisher.rs      # AMQP publisher (port 5672, publisher confirms + mandatory)
│   │   ├── models.rs             # 数据模型
│   │   ├── storage.rs            # sled 本地存储 + keyring 密码加密
│   │   ├── health.rs             # R6 健康度四态判定
│   │   ├── trace.rs              # R7 消息状态追踪（推断式）
│   │   ├── state.rs              # 全局 AppState
│   │   └── error.rs              # 统一错误类型
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── capabilities/default.json
│   └── icons/                    # 应用图标
├── src/                          # 前端
│   ├── main.jsx                  # Preact 入口
│   ├── app.jsx                   # 主应用（窗口外壳 + 路由 + 状态）
│   ├── components/               # 复用组件
│   │   ├── TitleBar.jsx          # 标题栏（自定义窗口控制）
│   │   ├── Sidebar.jsx           # 侧边栏导航
│   │   ├── Term.jsx              # 术语悬浮提示（R2 全局）
│   │   ├── Badges.jsx            # HealthBadge + StatusPill
│   │   ├── RateChart.jsx         # 速率 SVG 折线图
│   │   ├── ConfirmDialog.jsx     # 二次确认弹窗
│   │   ├── ConnectingOverlay.jsx # 连接骨架屏（液态玻璃 shimmer）
│   │   └── Toast.jsx             # 轻提示
│   ├── views/                    # 页面视图
│   │   ├── ConnectionsView.jsx   # R1 连接管理
│   │   ├── OverviewView.jsx      # R2 总览
│   │   ├── QueuesView.jsx        # R3 队列列表
│   │   ├── QueueDetailView.jsx   # R4 队列详情 + R6 健康度
│   │   ├── MessagesView.jsx      # R5 发送 + R7 消息流
│   │   └── SettingsView.jsx      # 设置 + 术语表
│   ├── lib/
│   │   ├── api.js                # Tauri 命令调用封装
│   │   ├── terms.js              # 术语映射表（PRD §6.1）
│   │   └── toast.js              # Toast 全局信号
│   ├── styles/
│   │   ├── tokens.css            # 设计 Token（来自 DesignSystemManifest）
│   │   └── glass.css             # 液态玻璃全局样式
│   └── tests/
│       └── setup.js              # Vitest 环境
├── index.html
├── package.json
├── vite.config.js
├── MQDesk_PRD.md                 # 产品需求文档
├── DesignSystemManifest.md       # 设计 Token 配套
├── DevHandoff.md                 # 开发者交接指南
├── prototype-liquid-glass.html   # 视觉/交互基准原型
├── AGENTS.md                     # Agent 工作指南
└── legacy/                       # 旧 Go/React 项目归档（不参与构建）
```

## 开发

### 环境要求

- Node.js 18+
- Rust 1.77+（含 cargo）
- WebView2 Runtime（Windows 10/11 通常已内置）

### 安装与运行

```bash
# 安装前端依赖
npm install

# 开发模式（Vite 热重载 + Tauri 桌面壳）
npm run tauri:dev

# 生产构建（出 msi 安装包）
npm run tauri:build

# 仅前端开发（浏览器预览，无 Tauri 命令）
npm run dev

# 前端测试
npm test

# Rust 后端测试
cd src-tauri && cargo test
```

### 端口说明（PRD §2）

- **5672**：AMQP 协议端口（生产者/消费者用，由 Rust 侧 lapin 客户端使用）
- **15672**：Management HTTP API 端口（总览、队列列表、抓取预览等管控操作，由 Rust 侧 reqwest 使用）

前端不直接连接 RabbitMQ，所有通信经 Tauri 命令调 Rust 后端，密钥不暴露在前端。
页面示例
<img width="1177" height="756" alt="image" src="https://github.com/user-attachments/assets/4312ef34-56a4-487b-86a6-e69767ac2dc7" />

## MVP 范围（按 PRD R1-R7）

- ✅ R1 连接管理：新建/测试/保存/编辑/删除，密码本地加密（keyring）
- ✅ R2 总览仪表盘：一句话健康度 + 4 统计卡 + 告警入口 + 全局中文 + 术语悬浮
- ✅ R3 队列列表：搜索/排序/健康色/行点击进详情
- ✅ R4 队列详情：健康色块 + 速率 SVG 图 + 抓取预览（requeue=true，不真正消费）
- ✅ R5 引导式发送：直发/交换机切换 + JSON 校验 + 二次确认 + 预判提示
- ✅ R6 健康度四态：正常/堆积预警/无人消费/空闲
- ✅ R7 消息通知列表：时间流 + 状态药丸 + 筛选 + 推断式状态追踪

## 设计 Token

完整 Token 见 [DesignSystemManifest.md](./DesignSystemManifest.md) 和 [src/styles/tokens.css](./src/styles/tokens.css)。

关键调整：
- 辅助色由原紫色 `#bf5af2` 替换为友好青绿 `#12b5a6`（规避紫色禁用）
- Latin 字体由 SF Pro 替换为 Plus Jakarta Sans（Google Fonts 可加载）
- 中文走 Windows 原生微软雅黑

## 项目背景

本项目重构自原 `RabbitConsumerHub`（Go + React B/S 架构），按 PRD v1.0 重定义为面向小白的桌面客户端。旧代码归档在 `legacy/` 子目录，不参与构建，仅作参考。
