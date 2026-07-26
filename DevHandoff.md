# MQDesk 开发者交接指南（技术栈建议 + 交接清单）

> 配套文件：`MQDesk_PRD.md`（需求）、`prototype-liquid-glass.html`（视觉/交互基准）、`DesignSystemManifest.md`（设计 Token）。
> 本文解决一件事：**怎么把上面三份"变成"一个 Windows x64 / ARM64 桌面程序。**

---

## 1. 技术栈推荐：Tauri 2.x

| 维度 | 推荐 | 理由 |
|---|---|---|
| 桌面壳 | **Tauri 2.x**（Rust + 系统 WebView2） | ARM64 原生支持好、安装包 ~10MB（Electron 通常 100MB+）、内存占用低、Rust 侧管连接与本地存储更安全 |
| 前端 | 原型是原生 HTML/CSS/JS，可直接复用；若团队熟可用 **Preact / Svelte** 轻框架（不强求） | 液态玻璃纯 CSS 即可，无需重框架 |
| 与 RabbitMQ 通信 | 前端经 Tauri 命令调 Rust 侧；Rust 用 `lapin`（AMQP 客户端）或 `reqwest` 走 Management HTTP API | 密钥/连接不暴露在前端 |
| 本地存储 | Rust 侧用 `sled` / 配置文件存连接配置 | 连接凭据不进前端 localStorage |
| 安装包 | `tauri-plugin-msi` / `wix` 出 `.msi`，CI 同时打 **x64 与 arm64** 两个产物 | 满足 PRD "x64 兼容 ARM64" |

**备选方案**（如团队有既定偏好）：
- **Electron**：最省事、生态全，但包体大、ARM64 构建需额外配置、内存高——小白工具不推荐。
- **WebView2 + C# / C++**：最"正统 Windows"，但原生壳要自己写，工作量最大。
- **Qt / .NET MAUI**：跨平台但和现有 HTML 原型复用度低。

> ⚠️ **决策点**：Tauri 是推荐项，但需你或 dev 最终拍板。不定，项目脚手架不敢建。

---

## 2. 端口澄清（重要，避免 dev 踩坑）

PRD 连接表单默认 `5672`，但那是 **AMQP 协议端口**（生产者/消费者用）。
**管控类操作（总览、队列列表/详情、发送消息、消息通知）实际走 Management HTTP API，端口 `15672`**。

原型为简化体验将两者合并展示。dev 实现时：
- 连接配置保存 **两个端点**（AMQP 5672 + Management 15672），或统一用 Management API 完成全部管控（发送消息也可用 Management API 的 `publish` 接口）。
- 错误映射要覆盖：连接拒绝、认证失败、vhost 不存在、无 Management 插件。

---

## 3. 目录结构建议

```
mqdesk/
  src-tauri/              # Rust 壳
    src/main.rs           # 窗口 + Tauri 命令（connect / listQueues / publish ...）
    Cargo.toml
    tauri.conf.json       # 窗口尺寸、无原生边框（用自定义标题栏，见原型）
  src/                    # 前端（可直接复用原型结构）
    index.html
    styles/
      tokens.css          # ★ 从 DesignSystemManifest 落地（见 §4）
      glass.css           # 液态玻璃面板/光晕
    views/
      connections.js  overview.js  queues.js  queueDetail.js  messages.js  settings.js
    app.js                # 路由 + 状态（原型里的 go() / renderX() 拆出）
  package.json
```

---

## 4. 设计 Token 怎么落成全局 CSS

`DesignSystemManifest.md` 里的变量 → `src/styles/tokens.css` 的 `:root`（原型里已内联，**可直接拷出**）：

```css
:root{
  --primary:#0a84ff; --primary-soft:#e6f0ff;
  --accent:#12b5a6;  --accent-soft:#e2f7f4;
  --ink-900:#1b2230; --ink-600:#475068; --ink-400:#6a7388; /* 见 Manifest 实际值 */
  --bg:#eaf0f7;
  --r-lg:18px; --r-md:14px; --r-sm:10px; --r-pill:999px;
  --s-1:4px; --s-2:8px; --s-3:12px; --s-4:16px; --s-5:24px; --s-6:32px; --s-8:48px;
  --font-display:"Microsoft YaHei","PingFang SC",sans-serif;
  --font-mono:"DM Mono",ui-monospace,monospace;
  --shadow-1:0 8px 24px rgba(30,50,90,.12);
}
```

- **字体**：Plus Jakarta Sans / DM Mono 用 `@font-face` 打包进 `src/`（不依赖网络，离线可用）；中文走系统雅黑。
- **液态玻璃**：`backdrop-filter: blur(20px)` + 半透明白 + 彩色光晕层。WebView2（Windows）支持正常；ARM64 上无差异。
- **动效**：全局过渡 `0.2s ease`；必须包 `prefers-reduced-motion` 降级（原型已处理）。

---

## 5. R1–R7 原型验证状态（dev 接手清单）

| 需求 | 原型状态 | dev 待办 |
|---|---|---|
| R1 连接管理（新建/测试/保存） | ✅ 交互已验证 | 接真实 AMQP/HTTP；凭据本地加密存储 |
| R2 总览仪表盘 | ✅ 布局/健康度已验证 | 真实指标拉取 + 自动刷新 |
| R3 队列列表 | ✅ 已验证 | 分页/排序/搜索（大数据量） |
| R4 队列详情 | ✅ 含 SVG 速率图 | 真实速率数据 + 抓取预览分页 |
| R5 引导式发送 | ✅ 直发/交换机切换、二次确认、toast 已验证 | 真实 publish；JSON 校验保留 |
| R6 消息通知列表 | ✅ 状态筛选已验证 | 真实消费状态回流 |
| R7 术语悬浮解释 | ✅ 全局已验证 | 复用 `.term` 组件，术语表可配置 |

> 范围提醒：原型只覆盖 **R1–R7（MVP）**。R8–R15（实时监听、桌面告警、新建向导、删除保护、P2 拓扑/测试场景/Diff 等）尚未出高保真，需另排期或先约定"先交付 MVP"。

---

## 6. 交接清单（发包前勾选）

- [ ] `MQDesk_PRD.md`
- [ ] `prototype-liquid-glass.html`（视觉/交互基准）
- [ ] `DesignSystemManifest.md`（设计 Token，必带）
- [ ] 本指南 `DevHandoff.md`
- [ ] **技术栈决策确认**（Tauri？备选？）
- [ ] **端口澄清确认**（5672 AMQP vs 15672 Management API，见 §2）
- [ ] ARM64 构建流水线（CI 出 x64 + arm64 双 msi）
- [ ] 错误处理映射表（连接/认证/vhost/权限）

---

## 7. 给 dev 的一句话开场

> "这是个面向小白的 RabbitMQ 桌面管控台，液态玻璃风格，Tauri 壳。先看 `prototype-liquid-glass.html` 知道长什么样，再照 `DesignSystemManifest.md` 建 `tokens.css`，管控操作走 15672 Management API。R1–R7 已验证交互，先做 MVP。"
