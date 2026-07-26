# ADR 003: 设计 Token 与液态玻璃风格

- 状态：Accepted
- 日期：2026-07-26

## 决策

1. 液态玻璃视觉以 `src/styles/tokens.css` 为唯一 Token 源，`src/styles/glass.css` 为全局样式层。
2. 核心变量 `--primary`、`--glass`、`--sh-1/2/3` 等不得修改、移除或重命名。
3. 新增 Token 需在 `DesignSystemManifest.md` 中记录并同步到 `tokens.css`。
4. 暗黑主题通过 `[data-theme="dark"]` 实现，由 `src/lib/theme.js` 管理。
5. 前端框架锁定为 Preact；禁止引入 React/Vue/Svelte 替代。

## 后果

- 修改核心 Token 会破坏整站视觉一致性；替换前端框架将破坏现有组件与构建配置。
