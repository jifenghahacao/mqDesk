# AGENTS.md

## 路由指引
- 仓库根：`/`
- Rust 桌面壳入口：`/src-tauri/src/main.rs` -> `/src-tauri/src/lib.rs`
- Tauri 命令：`/src-tauri/src/commands/`
- 核心库入口：`/src-tauri/crates/core/src/lib.rs`
- RabbitMQ 客户端：`/src-tauri/crates/core/src/rabbit/`
- 本地存储与加密：`/src-tauri/crates/core/src/storage.rs`、`crypto.rs`
- 健康度判定：`/src-tauri/crates/core/src/health.rs`
- 前端入口：`/src/main.jsx` -> `/src/app.jsx`
- 前端视图：`/src/views/`
- 前端组件：`/src/components/`
- 前端 API 封装：`/src/lib/api.js`
- 设计 Token：`/src/styles/tokens.css`
- 液态玻璃样式：`/src/styles/glass.css`
- Tauri 配置：`/src-tauri/tauri.conf.json`
- ADRs：`/docs/adr/`
- 旧项目归档：`/legacy/`

## 禁止事项
- 禁止：Tauri 命令层写业务规则 → 见 `/docs/adr/001-项目架构决策.md`
- 禁止：绕过 Tauri 命令直接从前端调用 RabbitMQ → 见 `/docs/adr/002-security-boundary.md`
- 禁止：修改或移除液态玻璃核心 CSS 变量 → 见 `/docs/adr/003-design-tokens.md`
- 禁止：用 React/Vue/Svelte 替换 Preact → 见 `/docs/adr/003-design-tokens.md`
- 禁止：在 `legacy/` 子目录修改任何文件 → 见 `/docs/adr/004-legacy-freeze.md`

## 最小构建/测试/部署命令
- 安装前端依赖：`npm install`
- 前端测试：`npm test`
- 前端构建：`npm run build`
- 后端测试：`cd src-tauri && cargo test`
- 后端构建：`cd src-tauri && cargo build`
- 开发环境：`npm run tauri:dev`
- 生产构建：`npm run tauri:build`
- 生成图标：`npx @tauri-apps/cli icon src-tauri/icons/icon.png`
