# DesignSystemManifest

> 来源：美学启动套件 - 模板 6「液态玻璃 Liquid Glass」
> 微调：将模板原紫色辅助色 `#bf5af2` 替换为友好青绿 `#12b5a6`（规避强禁止清单：紫色 / 蓝紫渐变）；Latin 字体由不可商用的 SF Pro 替换为 Plus Jakarta Sans（Google Fonts 可加载）；中文走 Windows 原生微软雅黑。
> 桌面外壳：完整 Windows 窗口（标题栏 + 最小化/最大化/关闭 + 左侧导航），背景含柔和彩色光晕。

---

## 1. 配色方案（命名色值，oklch + hex 双标注）

### 环境底色
| 名称 | 用途 | HEX | OKLCH |
|------|------|-----|-------|
| `--bg-base` | 应用环境底色（冷调浅灰） | `#eef1f8` | `oklch(0.95 0.012 255)` |
| `--bg-halo-1` | 左上角光晕（天蓝） | `#cfe0ff` | `oklch(0.88 0.06 250)` |
| `--bg-halo-2` | 右下角光晕（青绿） | `#c7f0e8` | `oklch(0.90 0.06 185)` |
| `--bg-halo-3` | 中部柔光（暖珊瑚，极淡） | `#ffe3da` | `oklch(0.91 0.05 35)` |

### 文字
| 名称 | 用途 | HEX | OKLCH |
|------|------|-----|-------|
| `--ink-900` | 主标题 / 关键数字 | `#1b2230` | `oklch(0.27 0.02 260)` |
| `--ink-600` | 正文 | `#565f73` | `oklch(0.50 0.02 260)` |
| `--ink-400` | 次要说明 / 占位 | `#6a7388` | `oklch(0.49 0.015 260)` |

> 微调记录：原型 QA 阶段将 `--ink-400` 由 `#8b94a7` 调暗至 `#6a7388`，使次要文字在白/玻璃底上的对比度达 WCAG AA（≥4.5:1），同色相仅调明度，未引入新色相。
| `--ink-on-accent` | 强调色上的文字 | `#ffffff` | `oklch(1 0 0)` |

### 品牌 / 强调
| 名称 | 用途 | HEX | OKLCH |
|------|------|-----|-------|
| `--primary` | 主操作 / 链接 / 选中 | `#2f7ff2` | `oklch(0.63 0.17 255)` |
| `--primary-soft` | 主色淡底（选中项背景） | `#e2edff` | `oklch(0.93 0.04 255)` |
| `--accent` | 辅助强调（青绿，替代原紫） | `#12b5a6` | `oklch(0.68 0.10 187)` |
| `--accent-soft` | 辅助色淡底 | `#d6f3ee` | `oklch(0.93 0.04 187)` |

### 语义状态（健康度四态 + 通用）
| 名称 | 含义 | HEX | OKLCH |
|------|------|-----|-------|
| `--ok` | 正常 / 已被消费 | `#1fa971` | `oklch(0.63 0.13 155)` |
| `--warn` | 堆积预警 | `#e8a33d` | `oklch(0.74 0.12 75)` |
| `--danger` | 无人消费 / 失败 / 危险操作 | `#e5484d` | `oklch(0.60 0.21 25)` |
| `--idle` | 空闲 / 中性 | `#9aa3b2` | `oklch(0.68 0.015 260)` |
| `--info` | 提示 / 进行中 | `#2f7ff2` | `oklch(0.63 0.17 255)` |

### 玻璃层 / 描边
| 名称 | 用途 | 值 |
|------|------|-----|
| `--glass` | 常规玻璃面板 | `rgba(255,255,255,0.62)` |
| `--glass-strong` | 强玻璃（弹窗 / 顶栏） | `rgba(255,255,255,0.82)` |
| `--glass-border` | 玻璃描边高光 | `rgba(255,255,255,0.75)` |
| `--hairline` | 发丝分隔线 | `rgba(27,34,48,0.08)` |

---

## 2. 字体配对

| 角色 | 字体 | 说明 |
|------|------|------|
| Display（标题 / 数字） | `"Plus Jakarta Sans"`, `"Microsoft YaHei"`, sans-serif | 几何友好，数字清晰；中文回退微软雅黑 |
| Body（正文） | `"Plus Jakarta Sans"`, `"Microsoft YaHei"`, sans-serif | 同上 |
| Mono（速率 / 计数 / 路由键） | `"DM Mono"`, `"JetBrains Mono"`, ui-monospace, monospace | 等宽数字，强化"管控台"技术感 |
| 字号梯度 | 12 / 13 / 14 / 16 / 20 / 24 / 28 / 32 / 40 / 56 | 序号梯度，页面标题 28、统计数字 32、大标题 56，正文 14，辅助 12–13 |

> 中文在 Windows 上由微软雅黑承载；Latin 与数字由 Plus Jakarta Sans / DM Mono 承载。加载失败时回退 `system-ui` / `ui-monospace`，不落入 Inter/Roboto/Arial。

---

## 3. 间距标尺（8px 基础）

`--s-1:4` · `--s-2:8` · `--s-3:12` · `--s-4:16` · `--s-5:24` · `--s-6:32` · `--s-7:48` · `--s-8:64`
- 卡片内边距默认 `--s-5`(24)；卡片间距 `--s-4`(16)
- 区块间距 `--s-6`(32)；页面左右留白 `--s-6`(32)
- 行高：正文 1.6；标题 1.25；数字 1

---

## 4. 圆角

`--r-sm:10` · `--r-md:14` · `--r-lg:20` · `--r-pill:999`
- 玻璃面板 / 卡片：`--r-lg`(20)
- 按钮 / 输入 / 标签：`--r-sm`(10) 或 `--r-md`(14)
- 状态药丸 / 头像：`--r-pill`(999)
- 窗口外壳圆角：`--r-md`(14)（仅外层，内部面板 20）

---

## 5. 阴影（柔和分层）

`--sh-1: 0 2px 8px rgba(27,34,48,0.06)` —— 卡片静息
`--sh-2: 0 8px 24px rgba(27,34,48,0.10)` —— 卡片悬浮 / 玻璃面板
`--sh-3: 0 16px 48px rgba(27,34,48,0.14)` —— 弹窗 / 浮层
`--sh-accent: 0 8px 24px rgba(47,127,242,0.28)` —— 主按钮强调投影
`--ring: 0 0 0 3px rgba(47,127,242,0.30)` —— focus 焦点环

---

## 6. 玻璃层 / 卡片样式

- 玻璃面板：`background: var(--glass)` + `backdrop-filter: blur(22px) saturate(165%)` + `border:1px solid var(--glass-border)` + `box-shadow: var(--sh-2)`，圆角 `--r-lg`
- 强玻璃（标题栏 / 弹窗）：`background: var(--glass-strong)` + `backdrop-filter: blur(30px) saturate(180%)`
- 彩色光晕：在窗口最底层放 3 个绝对定位径向渐变圆（halo-1/2/3），模糊 80–120px，营造"光透过玻璃"的签名感
- 卡片悬浮：translateY(-2px) + shadow 升至 `--sh-2`，过渡 `0.2s ease`

---

## 7. 组件清单（含完整交互状态）

### 7.1 窗口外壳 WindowShell
- 外层容器：固定 1180×760（桌面参考尺寸），圆角 14，整体投影 `--sh-3`
- 标题栏（强玻璃）：左侧应用名「MQDesk」+ 交通灯替换为 **Windows 控件**（最小化 / 最大化 / 关闭，右侧排列，非 macOS 红黄绿）
- 左侧导航栏（玻璃）：Logo + 导航项（连接 / 总览 / 队列 / 消息 / 设置）

### 7.2 导航项 NavItem
- default：文字 `--ink-600`，左图标线性，无背景
- hover：背景 `rgba(27,34,48,0.05)`，文字 `--ink-900`
- active：背景 `var(--primary-soft)`，文字 `var(--primary)`，左侧 3px 主色条
- focus：焦点环 `--ring`
- disabled：文字 `--idle`，不可点

### 7.3 按钮 Button
- 变体：primary（实心主色）/ secondary（玻璃描边）/ ghost（纯文字）/ danger（实心危险红）
- default：primary 背景 `--primary` 文字白；hover：亮度 +6% + `--sh-accent`；active：translateY(1px)；focus：`--ring`；disabled：背景 `--idle` 透明度 40% 文字白 60%；loading：左内显示 spinner，文字「处理中…」
- 尺寸：sm(32h) / md(40h) / lg(48h)；圆角 `--r-sm`

### 7.4 输入框 InputField
- 结构：标签（含可选「?」术语提示）+ 输入框 + 辅助说明 / 错误
- default：玻璃底，1px `--hairline` 描边；focus：描边 `--primary` + `--ring`；error：描边 `--danger` + 错误文字 `--danger`；disabled：底 `--idle` 10%
- 术语提示「?」：hover/focus 显示气泡，文案来自 §6.1 中文化映射

### 7.5 玻璃卡片 Card
- 静息 `--sh-1`；hover `--sh-2` + translateY(-2px)；内含标题行 + 内容区

### 7.6 统计卡 StatCard（总览）
- 大号数字（Mono，32–40）+ 标签 + 可选趋势小标；点击可跳转（告警卡跳队列详情）

### 7.7 健康度徽标 HealthBadge
- 四态：🟢正常(ok) / 🟡堆积预警(warn) / 🔴无人消费(danger) / ⚪空闲(idle)
- 呈现：色点 + 文字药丸；大号版本用于队列详情顶部色块

### 7.8 状态药丸 StatusPill（消息命运）
- 已发送(primary) / 已被消费(ok) / 仍堆积(warn) / 消费失败(danger)
- 文字 + 同色淡底

### 7.9 表格 / 列表行 Row
- 队列列表行：hover 背景 `--primary-soft` 10%；选中态左侧主色条；行内健康点

### 7.10 弹窗 Dialog（二次确认）
- 强玻璃 + `--sh-3`；遮罩 `rgba(27,34,48,0.28)` 模糊；标题 + 说明（含「这一步在做什么」一句话）+ 操作按钮（取消 / 确认-危险红）；出现动画 `0.2s`
- 危险操作额外显示「⚠ 此操作会真实生效，且不可撤销」

### 7.11 轻提示 Toast
- 顶部居中浮层，玻璃卡 + 图标 + 文案；成功绿 / 失败红；自动消失 3s；出现 `0.2s`

### 7.12 术语提示 Tooltip
- 触发「?」图标；浮动气泡，白底 + `--sh-2` + 文字 `--ink-600`；最大宽 240px

### 7.13 分段控制 / 标签页 Segmented
- 选项等宽，选中填充 `--primary-soft` + 文字 `--primary`；切换 `0.2s`

### 7.14 迷你图表 MiniChart
- 速率曲线（进/出双线 SVG），线宽 2，进线 `--primary` 出线 `--accent`；坐标淡网格；hover 显示数值点

---

## 8. 动效规范
- 全局过渡 `0.2s ease`（与 QA 终检 0.2–0.3s 一致）
- 尊重 `prefers-reduced-motion`：关闭位移与模糊动画，仅保留透明度变化
- 悬停目标 ≥ 44px；缩放 / 浮层用 transform + opacity，避免重排

---

## 8.1 组件补充：连接骨架屏 ConnectingOverlay（迭代 #6 新增）
- 触发：点击连接卡 / 保存并连接后，覆盖整个 `.body`（含侧栏），z-index 30
- 结构：标题骨架 + 横幅骨架 + 4 张统计卡骨架（shimmer 流光）+ 状态行「正在连接「名称」…」+ 旋转 spinner
- 动效：`shimmer` 流光 1.2s 循环；显示 `0.2s` fade；停留 ~900ms 后淡出进入总览
- 颜色：骨架用白玻璃渐变（非新色相），与底座 `--bg-base` 一致

## 9. 可访问性基线（迭代 #6 确立）
- **skip-link**：页面首个可聚焦元素，视觉移出屏幕，`:focus` 时滑入；锚点 `#mainContent`（`main` 设 `tabindex="-1"`）
- **语义化点击**：所有带 `onclick` 的非按钮元素（连接卡 / 告警行 / 队列行 / 文字链接）统一补 `role="button"` + `tabindex="0"` + Enter/Space 触发（由 `makeKeyboardAccessible()` 统一处理，遮罩 `.scrim` 排除）
- **表单关联**：每个 `<label>` 用 `for` 指向对应控件 `id`；无单一控件的控件组（如发送方式分段）以语义说明替代
- **焦点可见**：全局 `:focus-visible` 焦点环 `--ring`；可点击非按钮元素补 `:focus-visible` 同款焦点环
- **键盘完整**：导航 / 按钮 / 弹窗 / 可点击卡片均键盘可达；模态 `role="dialog"`+`aria-modal` 已具备
- **动效偏好**：`prefers-reduced-motion` 关闭位移与模糊，仅保留透明度
