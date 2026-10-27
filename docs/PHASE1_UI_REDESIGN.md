# Phase 1.1B-1R — Editorial Instrument

2026-10-11，macOS arm64。真实 React / Tauri 应用的视觉重设计已实现并生成可运行 `.app`，等待用户视觉批准。未创建独立设计实验，未进入 Phase 2，未提交 Git。完整 DMG 封装失败，不能称为完整分发构建通过。

## 范围与原有工作

工作目录为 `<repository root>`，分支为 `codex/phase1-ui-implementation`。开始和交付时 HEAD 均为 `6821805a52989e374ca2c10d5bd1cf225c5d8c25`。原有未提交和未跟踪的生产 UI 工作已保留；开始前保存了工作状态归档，范围检查以该归档为基线，而不是把全部 Git diff 当成本轮成果。

[范围证明](../.impeccable/review/phase1-1r/scope-proof.md)和[哈希记录](../.impeccable/review/phase1-1r/scope-proof.json)确认：Rust 权限与行为、typed IPC、fixture、路由、偏好实现、依赖与锁文件、字体及许可均未改变。没有 Discovery、安装、认证、凭证、真实 Health、配置写入或 Safe Repair 操作。

## 研究与设计决策

实际浏览并检查了 Apple、Pentagram / Isomorphic Labs、Pi、Resend，以及 Linear、Things 的渲染画面。研究包含 17 张截图；各站的具体观察、准确 URL、未加载媒体和交互限制见[视觉研究记录](../.impeccable/review/phase1-1r/visual-research.md)。没有采用网站的代码、导航、商标、图形或产品流程。Teenage Engineering 未纳入研究。

编辑生产 UI 前写下了[方向备忘录](../.impeccable/review/phase1-1r/art-direction.md)。最终落地数值由 [DESIGN_SYSTEM](DESIGN_SYSTEM.md)维护；备忘录保留编辑前的决策过程。

| 具体改变 | 对 Nexus 的作用 |
| --- | --- |
| 168px 导航栏，紧凑桌面 152px；标题式 Nexus 字标和短基线手势 | 增加工作区域，替换原有粉色 N，保留小尺寸辨识度供用户评判 |
| Bricolage 用于拉丁身份，思源黑体用于中文，Plex Mono 用于精确标识 | 通过阅读角色建立层级；中文标题保持零字距，辅助文字通常 13–14px |
| Home 使用开放 Agent 列表和中性解释栏 | 四个 Agent 成为主视觉；状态、原因和下一步清晰，减少大面积填色容器 |
| 两条行内阅读轴；状态和安装信息在右侧；快捷入口紧接列表 | 提高桌面密度，同时保留完整模型 ID、分配来源和可访问描述 |
| 问题解释后立即出现主要行动，再列生效模型和版本记录 | 降低寻找下一步的成本；不混淆预期方案、已应用记录与当前生效来源 |
| Agent Detail 移除巨型外框和嵌套模型卡片 | 身份、就绪状态、行动、配置和健康依据形成连续工作区 |
| Light 冷灰画布与浅导航；Dark 蓝灰画布与更深导航；蓝色只强调交互 | 外观有明确材料关系，健康状态仍由语义色、图标和文本共同表达 |
| System 使用共同字体、控件、状态和开放分隔线 | 保留已有列表结构，与主要页面形成一致的产品语言 |

## 实际原生迭代

本会话附件只有任务文本，没有三张图片文件。未声称检查过缺失图片；改为截图记录实际运行的旧版应用作为可用基线。

| 页面 | 原生基线 | 最终 Dark | 最终 Light |
| --- | --- | --- | --- |
| Home / zh-CN | [之前](../.impeccable/review/phase1-1r/native-before/home-zh-CN-system-1200x800.jpg) | [之后](../.impeccable/review/phase1-1r/native-final/home-zh-CN-dark-1200x800.jpg) | [之后](../.impeccable/review/phase1-1r/native-final/home-zh-CN-light-1200x800.jpg) |
| Claude Code / zh-CN | [之前](../.impeccable/review/phase1-1r/native-before/agent-claude-zh-CN-system-1200x800.jpg) | [之后](../.impeccable/review/phase1-1r/native-final/agent-claude-zh-CN-dark-1200x800.jpg) | [之后](../.impeccable/review/phase1-1r/native-final/agent-claude-zh-CN-light-1200x800.jpg) |
| System / zh-CN | [之前](../.impeccable/review/phase1-1r/native-before/system-zh-CN-system-1200x800.jpg) | [之后](../.impeccable/review/phase1-1r/native-final/system-zh-CN-dark-1200x800.jpg) | 本轮未要求该原生组合 |
| Home / en-US | — | [之后](../.impeccable/review/phase1-1r/native-final/home-en-US-dark-1200x800.jpg) | [之后](../.impeccable/review/phase1-1r/native-final/home-en-US-light-1200x800.jpg) |

这些截图来自打包应用 `tauri://localhost`，逻辑窗口 1200×800，原始图像 2400×1600。[原生截图清单](../.impeccable/review/phase1-1r/native-manifest.json)记录原始尺寸、图像哈希与最终应用哈希。原生标题栏中的紫色共享指示器和截图中的指针属于 macOS / 截图过程，不是 Nexus 品牌元素。

第一轮真实截图仍有三层 Agent 文字，快捷入口被较长解释栏向下推，行动位于技术记录之后。第二轮删除重复厂商行、重排信息和行动。再次复核原生英文画面时发现分配来源间距被 CSS 优先级压缩，修正后重新打包、截图并复核。第一轮证据保留在 `native-round1/`。

补充原生证据包括[键盘焦点与选中状态](../.impeccable/review/phase1-1r/native-final/home-zh-CN-dark-keyboard-focus.jpg)、[1470×923 放大窗口](../.impeccable/review/phase1-1r/native-final/home-zh-CN-dark-1470x923.jpg)，以及 [Agents](../.impeccable/review/phase1-1r/native-final/agents-zh-CN-dark-1200x800.jpg)、[Models](../.impeccable/review/phase1-1r/native-final/models-zh-CN-dark-1200x800.jpg)、[Extensions](../.impeccable/review/phase1-1r/native-final/extensions-zh-CN-dark-1200x800.jpg)、[Settings](../.impeccable/review/phase1-1r/native-final/settings-zh-CN-dark-1200x800.jpg) 的共享系统复核。

角点和边缘拖动没有改变原生窗口尺寸；窗口菜单的“缩放窗口”只能得到观察到的 1470×923。现有原生控制接口没有指定矩形尺寸的方法。因此 **1440×900、1280×800、960×640 尚未取得精确原生证据**。这些尺寸已有 Home / Claude / System × Light/Dark × zh-CN/en-US 的 36 张[浏览器截图及清单](../.impeccable/review/phase1-1r/browser-final/manifest.json)，它们不替代原生验证。

## 本轮实际生产文件

以下 8 个文件相对于开始前的工作状态发生变化：

- [src/styles.css](../src/styles.css)
- [src/components/NexusIdentity.tsx](../src/components/NexusIdentity.tsx)
- [src/components/AgentRows.tsx](../src/components/AgentRows.tsx)
- [src/pages/Home.tsx](../src/pages/Home.tsx)
- [src/pages/AgentDetail.tsx](../src/pages/AgentDetail.tsx)
- [src/pages/System.tsx](../src/pages/System.tsx)
- [src/locales/zh-CN/common.json](../src/locales/zh-CN/common.json)
- [src/locales/en-US/common.json](../src/locales/en-US/common.json)

测试改动仅为 [production-ui.spec.ts](../tests/ui/production-ui.spec.ts) 的尺寸、可见性与信息保留检查。更新了设计系统；旧 [B-1 实施记录](PHASE1_UI_IMPLEMENTATION.md)标记为已被本记录替代。原有 App、ui、领域文案、配置/证据组件及其他页面没有归为本轮新实现。

## 技术验证

| 执行命令 / 检查 | 实际结果 |
| --- | --- |
| `pnpm typecheck` | 通过，exit 0 |
| `pnpm exec eslint src tests vite.config.ts playwright.config.ts` | 最终生产范围通过，exit 0 |
| `pnpm test` | 4 文件、15 项通过，exit 0 |
| `pnpm build`，由 Tauri beforeBuild 执行 | 最终前端构建通过，exit 0 |
| `pnpm test:ui tests/ui/production-ui.spec.ts` | 最终 CSS：18 项通过，其中 12 个尺寸/外观/语言组合；36 张截图 |
| `pnpm test:ui tests/ui/shell.spec.ts` | 22 项通过；覆盖导航、主题/语言、keyboard、axe 主页面与子视图；最后两项 CSS 间距/小于899px表头修正未改变这些检查所用状态 |
| `pnpm native:check` | 通过，exit 0 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过，exit 0 |
| `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 通过，exit 0 |
| `pnpm native:test` | 3 项 Rust 测试通过，exit 0 |
| `pnpm desktop:build` | 前端、release 编译和 `.app` 生成成功；`bundle_dmg.sh` 失败，整体 exit 1，两轮均记录 |
| `pnpm desktop:build --bundles app` | 最终 `.app` 构建成功，exit 0；已实际启动并完成原生截图 |
| `pnpm lint` | 仍失败：归档 design-lab 中 258 个已有错误、0 warnings；生产范围无新增错误，未删除实验 |
| `git diff --check` | 通过 |
| Impeccable static detect | exit 0，空 findings；不是视觉批准 |

日志位于 [logs](../.impeccable/review/phase1-1r/logs/)。Playwright 的 Vite 服务在测试全部通过后需要清理本次测试创建的孤立进程，具体 PID 和 exit 0 记录保留在 notes JSON 中；初始开发服务中断导致的失败尝试也保留，未当作通过结果。

检查确认目标尺寸无横向溢出、关键控件文字可容纳、1440×900 下四个 Agent 与主要行动完整可见、状态包含文本和图标、焦点与选择不同、fixture 的原生分配和待确认版本语义准确。字体来自现有本地许可文件，浏览器有离线加载检查，打包原生应用已呈现它们。这些是最低功能证据，不是完整 WCAG 或审美认证。

## 剩余视觉与验证问题

- 字标和短基线手势仍较克制，是否足够鲜明需要用户判断。
- 1200×800 英文 Home 的健康依据接近下缘，System 和活动需要滚动；四个 Agent 与主要行动已在首屏。
- 36 张浏览器画面逐张复核未见关键文字裁切或控件重叠。960×640 下英文 Home 的分配信息增加换行；Claude 的配置记录与健康依据、System 的 Python/npm 需要垂直滚动。四个 Agent 与 Home 主要行动仍在首屏。
- System 与其他辅助页面保留常规列表结构，本轮仅统一共享视觉系统。
- 三个指定精确原生尺寸、Windows 渲染、VoiceOver / NVDA / Narrator 朗读、长期阅读舒适度尚未验证。
- DMG 封装未通过；本轮交付为本地可运行 `.app`，不是经过签名、公证和跨平台验证的发布包。

完整的 12 项人工审视见[最终艺术指导复核](../.impeccable/review/phase1-1r/final-art-direction-review.md)。没有声称达到世界级品质；用户保留最终审美权。

## 打开应用

最终原生应用已打开在 Home，语言恢复为简体中文，外观恢复为跟随系统。以后可以在 Finder 打开：

`<repository root>/src-tauri/target/release/bundle/macos/Project Nexus.app`

开发运行：在项目根目录执行 `pnpm desktop:dev`。仅构建可运行应用：`pnpm desktop:build --bundles app`。

本轮在交付处停止，等待明确视觉批准；不自动提交，也不开始下一阶段。
