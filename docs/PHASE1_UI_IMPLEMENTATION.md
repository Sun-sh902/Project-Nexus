# Phase 1.1B-1 生产桌面 UI 历史实施记录（视觉基线已被否决）

> 发布说明（2026-10-27）：本文链接的 `design-lab/` 与 `.impeccable/` 设计证据仅保存在本地开发环境，不包含在发布的仓库中。

历史记录日期：2026-10-11（Asia/Shanghai）。本文件保存 Phase 1.1B-1 当时的实施与验证证据，其生产视觉基线已被用户否决。当前获授权的 Phase 1.1B-1R / **Editorial Instrument** 视觉、原生截图、验证结果与剩余限制由 [PHASE1_UI_REDESIGN.md](PHASE1_UI_REDESIGN.md) 记录；当前视觉规格由 [DESIGN_SYSTEM.md](DESIGN_SYSTEM.md) 统一维护。旧字标、pink/purple 上下文、字号、间距、tokens、截图与测试计数不代表当前应用，也不构成审美批准。

**当前状态：本文件为保留的历史证据，已由 1.1B-1R 取代其视觉实施定义；新版本仍等待用户明确视觉批准。** 下文所有“当前”“本批次”“已完成”及结果均指历史 B-1 批次，不能转移为 B-1R 验证。B-1 当时已在现有 React / Tauri 2 桌面应用中实施共享视觉系统、外壳、Home 与 Agent Detail，业务数据全部来自 fixture。其独立评审最终 `ship` 仅针对 R1/R2 两项无障碍修正，不代表最终审美批准、完整无障碍认证或跨平台验收。保留 [当时最终评审](../.impeccable/review/phase1-ui/finish-review.md)、[当时验证记录](../.impeccable/review/phase1-ui/validation.json) 与 [当时验证限制](../.impeccable/review/phase1-ui/validation-limits.json)，不删除历史证据。

## 1. 工作区与授权范围

| 项目 | 当前记录 |
| --- | --- |
| 工作区 | `<repository root>` |
| 分支 | `codex/phase1-ui-implementation` |
| 当前 HEAD / 原始 UI 比较基线 | `6821805a52989e374ca2c10d5bd1cf225c5d8c25` |
| 产品 | AI Agent Lifecycle Manager；Nexus 管理其他 Agents 的生命周期，不替代 Agents |
| 视觉依据 | 用户指定的 [Joined Measure](../design-lab/design-lock/README.md)，作为暂定视觉基础 |
| 交互依据 | 用户指定的 [Continuity / 接续](../design-lab/visual-breakthrough/DIRECTION.md)，作为选择与上下文关系基础 |
| 本批次范围 | 全局外壳、共享 tokens / 字体 / 组件、Home、Agent Detail 五个既有区域 |

实现没有切换分支、创建 Git worktree、创建独立视觉原型或提交 Git commit。没有启动 Phase 2，没有新增依赖、系统字体安装、持久化迁移或真实 Agent 操作。

Home、Agents、Models、Extensions、System、Settings 六个主导航目的地和原有 hash 路由保持不变。Agents、Models、Extensions、System、Settings 的页面构成未重新设计；它们继承共享字体、tokens 和组件样式。Agent Detail 保留 Overview、Model、Extensions、Configuration、Diagnostics。产品、架构、安全、国际化与阶段顺序仍由 [AGENTS.md](../AGENTS.md)、[PRD](PRD.md)、[ARCHITECTURE](ARCHITECTURE.md)、[SECURITY](SECURITY.md)、[I18N](I18N.md)、[ROADMAP](ROADMAP.md) 约束。

按本次明确要求，Phase 0 规范及 [PHASE1.md](PHASE1.md) 未重写。其旧版视觉 / 系统字体描述是历史或原提案状态；本文件记录此次获授权的实现增量，不另立阶段路线图。Joined Measure 与 Continuity 都没有获得最终视觉批准。

## 2. 修改与新增文件

下表覆盖本批次全部生产代码、翻译、字体与相关测试文件；本文件为新增交付记录。没有修改业务 fixture、偏好实现、路由实现、Rust、Tauri 配置 / capabilities、依赖清单或锁文件。

| 文件 | 状态与原因 |
| --- | --- |
| [src/styles.css](../src/styles.css) | 修改：本地字体、独立 Light / Dark tokens、字级、外壳、Home 联动工作面、Detail、焦点与响应式样式 |
| [src/App.tsx](../src/App.tsx) | 修改：品牌、六项导航、Agent 数量、版本、折叠行为、紧凑位置栏及全局主题 / 语言控件 |
| [src/components/AgentRows.tsx](../src/components/AgentRows.tsx) | 修改：既有列表继承新字级；增加 Home AgentIndex、选择状态与完整本地化行描述 |
| [src/components/ui.tsx](../src/components/ui.tsx) | 修改：五种状态的图标与文字、可选标题描述、DetailLink 紧凑 / 主操作变体及名称与可见标签一致 |
| [src/components/NexusIdentity.tsx](../src/components/NexusIdentity.tsx) | 新增：项目既有中断式 N 与小写字标、双语产品说明、品牌无障碍名称 |
| [src/components/AgentConfiguration.tsx](../src/components/AgentConfiguration.tsx) | 新增：复用生效 Model / 来源 / Provider 与预期、已应用版本记录；保留无 Profile 情况 |
| [src/components/HealthEvidence.tsx](../src/components/HealthEvidence.tsx) | 新增：复用示例检查、未检测在线服务、固定依据时间与来源；不执行检测 |
| [src/pages/Home.tsx](../src/pages/Home.tsx) | 修改：单一就绪摘要、四 Agent 索引、选中 Agent 上下文、快捷入口、紧凑 System 与示例活动 |
| [src/pages/AgentDetail.tsx](../src/pages/AgentDetail.tsx) | 修改：身份 / 安装 / 版本 / 作用域、五个区域、配置关系、健康依据、真实页签导航动作与焦点转移 |
| [src/locales/zh-CN/agents.json](../src/locales/zh-CN/agents.json)、[src/locales/en-US/agents.json](../src/locales/en-US/agents.json) | 修改：选择与行描述、预期 / 已应用记录、版本、下一步动作、无快照 / 无字段差异说明 |
| [src/locales/zh-CN/common.json](../src/locales/zh-CN/common.json)、[src/locales/en-US/common.json](../src/locales/en-US/common.json) | 修改：准确简化演示提示、产品说明、由可见动作标签及 Agent 名组成的完整无障碍名称 |
| [src/locales/zh-CN/health.json](../src/locales/zh-CN/health.json)、[src/locales/en-US/health.json](../src/locales/en-US/health.json) | 修改：健康依据、示例检查范围、依据来源 |
| [src/locales/zh-CN/home.json](../src/locales/zh-CN/home.json)、[src/locales/en-US/home.json](../src/locales/en-US/home.json) | 修改：选择上下文与播报、概览入口、四 Agent 解释、准确的示例 System 摘要 |
| [public/fonts/bricolage.woff2](../public/fonts/bricolage.woff2)、[public/fonts/source-han.woff2](../public/fonts/source-han.woff2)、[public/fonts/plex-mono.woff2](../public/fonts/plex-mono.woff2) | 新增到生产资源：复用项目已有、已核验的三份本地候选字体原字节 |
| [public/fonts/bricolage.txt](../public/fonts/bricolage.txt)、[public/fonts/han.txt](../public/fonts/han.txt)、[public/fonts/plex-mono.txt](../public/fonts/plex-mono.txt)、[public/fonts/sources.json](../public/fonts/sources.json) | 新增：SIL OFL 1.1 许可与固定上游 revision、来源、字节数及 SHA-256；来源 URL 仅为溯源数据 |
| [tests/unit/contrast.test.ts](../tests/unit/contrast.test.ts) | 修改：加入 rail / elevated 表面与控制边界对比验证 |
| [tests/unit/fonts.test.ts](../tests/unit/fonts.test.ts) | 新增：本地字体字节 / hash / 许可 / 固定来源、CSS 引用及无远程字体 URL |
| [tests/ui/production-ui.spec.ts](../tests/ui/production-ui.spec.ts) | 新增：双语选择 / 配置语义、R1/R2、概览到 Model 的动作与焦点、折叠及持久偏好、字体覆盖 |
| [tests/ui/shell.spec.ts](../tests/ui/shell.spec.ts) | 修改：匹配完整的非紧凑详情链接名称，保留既有回归矩阵 |
| [docs/PHASE1_UI_IMPLEMENTATION.md](PHASE1_UI_IMPLEMENTATION.md) | 新增：本批次交付、证据、未验证项与 B-2 建议 |

审查与截图存放于 [`.impeccable/review/phase1-ui/`](../.impeccable/review/phase1-ui/)，不是生产 UI。完整变更清单见 [delivery-state.json](../.impeccable/review/phase1-ui/delivery-state.json)。文档阶段仅写入本文件，没有创建 `PRODUCT.md`、`DESIGN.md`、设计 sidecar 或 surface brief。

## 3. 视觉系统与桌面构成

### 独立主题

以 Joined Measure 的冷中性色、白色工作面、炭灰 Dark 与桑葚 / 玫瑰交互色为基础。品牌、当前位置、选择和下一步使用交互色；就绪仍使用独立绿色、赭色、红色语义。Unknown 与 Unsupported 由不同图标和明确文字区分，安装状态与就绪状态分开。

下列数值直接来自生产 [styles.css](../src/styles.css)，不是对截图颜色的估算。

| 语义 | Light | Dark | 用途 |
| --- | --- | --- | --- |
| canvas | `#f1f3f3` | `#1c2021` | 全局底面 |
| panel | `#ffffff` | `#252a2b` | 工作面 |
| rail | `#e7ebeb` | `#171b1c` | 稳定导航 |
| subtle / elevated | `#f6f7f7` / `#ffffff` | `#2b3132` / `#2b3132` | 生效 Model、支持信息与浮层底面 |
| primary / secondary text | `#252b2c` / `#596365` | `#edf0ef` / `#b2bcbc` | 主次信息 |
| subtle / control border | `#d6dcdc` / `#7a8587` | `#41494a` / `#859192` | 分组线与可辨识控制边界 |
| accent / focus | `#703b50` | `#e2adc0` | 导航、选择、动作、焦点 |
| on-accent | `#ffffff` | `#2d1c23` | 主按钮文字 |
| selected / hover | `#eee4e8` / `#e5e9e9` | `#44323b` / `#343c3d` | 当前上下文与指针反馈 |
| success | `#24664f` | `#9dd1ba` | Ready / 检查通过 |
| warning | `#835916` | `#e8c48c` | Needs Attention |
| danger | `#a3393f` | `#f1a7ab` | Blocked / 检查失败 |

Light 用稍深的导航和白色工作面建立层级；Dark 的导航比 canvas 深、工作面比 canvas 浅，生效 Model 再提高一级，主动作使用浅玫瑰底与深文字。Dark 另调字重与文本，不采用颜色反相。

界面以色面与 1px 分隔表达层级，没有新增装饰阴影、玻璃、金属图形或卡片网格。既有 Settings 的 Follow System 示例色块仍可表达两种主题。主要工作面圆角 10px，按钮 / 导航 / 生效 Model 为 6px，fixture 标签为 4px。键盘焦点为 2px 轮廓、4px 外偏移，与选择色面独立。

### 外壳与布局

正常侧栏 200px，1099px 及以下为 184px，手动折叠为 56px。导航保留完整名称、当前位置、工具提示和键盘访问；折叠是当前界面状态，未新增持久化。全局主题 / 语言控件移至 56px 紧凑位置栏，收起导航后仍可使用。主内容最大宽度 1280px，正常水平内边距 28px，内容在 `main` 内纵向滚动。

Home 采用一个共同工作面：左侧四 Agent、右侧当前解释，通过选中行的同色背景和 3px 选择边连通。一般列比为 1.4:1，1279px 及以下为 1.2:1，959px 及以下上下排列。Agent 行选择按钮与尾部详情链接是独立控件，没有嵌套交互。选择保留 Home 路由和当前焦点，上下文通过轻量 live region 播报。

Agent Detail 将身份与安装 / 版本 / 作用域放在页签上方，Overview 依次展示问题、有效 Model、配置记录、下一步和重要 Extensions。宽窗右侧健康依据为 256px；1179px 及以下移为主列中的显式展开项。技术标识允许换行，页签可折行；重要阅读文字保留 13–14px。

Tauri 的原生标题栏、窗口控制与现有初始 1200×800 / 可缩放配置保持不变，没有定制窗口 chrome。macOS 实际窗口已观察；Windows 标题栏行为仍未运行验证。

## 4. 字体与双语排版

三份字体通过本地 `@font-face` 载入，内部别名为 `Nexus Latin`、`Nexus Han`、`Nexus Mono`，分别对应 Bricolage Grotesque、Source Han Sans SC 候选（文件内部为 Source Han Sans CN VF）、IBM Plex Mono。正文栈先使用 Latin，再由 Han 覆盖中文，保留系统字体 fallback。启用 `font-synthesis: none`、光学尺寸与技术数字等宽；没有对中文统一添加 tracking。

| 角色 | 当前生产规格 |
| --- | --- |
| 小写字标 | 31px / 36px；Light 560、Dark 535；`opsz` 40、`wdth` 96；整体与 e/x/s 单字间距微调 |
| 页标题 / Home 就绪句 | 24px / 32px、480；较窄布局可为 22px / 30px |
| Agent 行名称 | 18px / 26px、460；Latin `opsz` 22，字距 `-.018em` |
| 问题标题 | 20px / 28px、470；较窄上下文为 19px / 27px |
| 正文 | 14px / 22px；Light 400、Dark 390；Detail 问题说明为 14px / 23px |
| 标签 / 状态 / 元数据 | 主要为 13px / 20–22px，状态 450；使用普通字重而非全面加粗 |
| Model ID / 版本 / 技术值 | Plex Mono Regular 400，通常 13px / 20px；聚焦 Model 为 18px / 27px，Home 上下文为 15px / 23px；修订号 22px / 30px |

字体来源与许可随包附带。Bricolage 为 204,636 字节，Source Han 为 7,711,988 字节，Plex Mono 为 49,248 字节，三份合计 7,965,872 字节（约 7.97 MB）。它们复用既有候选完整文件，本次没有做新子集化或系统安装。CSS 仅引用 `/fonts/*.woff2`，`font-display: swap`；生产 CSP 的 `font-src 'self'` 保持不变，运行时没有远程 CDN 字体请求。

字体单元测试核对固定 SHA-256、字节数、上游 revision 与 OFL 文本；Chrome 测试通过 `CSS.getPlatformFontsForNode` 验证当前中文资源全部去重 CJK 字符由本地 Source Han 渲染，并确认三字体加载及无远程字体请求。原生最终包的实际双语字体呈现已截图查看；这不代替 Windows 字体栅格化、完整原生 glyph 覆盖或长期阅读研究。

沿用 i18next、完整双语字符串与既有 Intl 数字 / 时间格式化。中英文实时切换保留当前 Detail 页签。Agent 名称和 Model ID 保持精确值；普通 UI 与无障碍标签来自 locale 资源。日期仍为固定 fixture 时间，不显示为刚刚发生的检测。

## 5. 保留的产品行为与配置真实性

[src/data/fixtures.ts](../src/data/fixtures.ts) 原样保留，为唯一业务数据边界。四 Agent 状态保持：

| Agent | 安装 | 就绪 | 生效 Model / 来源 | Profile 记录 |
| --- | --- | --- | --- | --- |
| OpenAI Codex | 已安装 | Ready | `demo-coding` / Model Profile | 已应用 v1，预期 v1 |
| Claude Code | 已安装 | Needs Attention | `demo-fast-coding` / Agent 原生配置 | 已应用记录 v1，预期 v2，待确认 |
| Gemini CLI | 未安装 | Blocked | 未配置 / Agent 原生配置模式 | 无预期或已应用版本记录 |
| OpenCode | 已安装 | Ready | `example/custom-model` / 直接 Model 分配 | 无预期或已应用版本记录 |

Home 的 2 Ready、1 Needs Attention、1 Blocked 来自这些固定数据。Gemini 的缺少安装没有改成 Unsupported。共享状态仍保留 Ready、Needs Attention、Blocked、Unknown、Unsupported 五种语义，没有数字 Health 分数。

Claude 的三类状态明确分开：预期是 `Coding — Fast` Profile 的 v2；已应用仅是 fixture 的 v1 修订记录；当前有效 Model 仍是来自 Agent 原生配置的 `demo-fast-coding`，其 Provider 为 Anthropic。Profile 更新不会自动改写 Agent，v1/v2 不是配置快照。没有 Profile 的 Agent 显示直接分配或原生模式及缺少记录，不伪造修订号。

“对比配置 / Compare configuration”实际打开现有 Model 页签并把焦点移至选中的页签，未创建 diff 工具。Model 与 Configuration 明示：fixture 仅含版本记录，没有保存的配置快照或字段级差异，未读取或应用任何配置。未安装 Agent 的下一步打开 Diagnostics 安装要求。Home 的主操作打开对应 Agent 的 Overview；其他入口导航至已有 Models / Extensions / System。安装、修复、凭据、模型应用等仍为预览 / 不可用状态，没有空点击冒充执行。

Health 依据只展示既有示例检查、通过 / 失败文字、未检测在线服务、固定时间 `2026-10-07T02:30:00Z` 与开发 fixture 来源。System 的工具 / OS / 架构和近期活动同样是演示数据，不是此次主机探测结果。全局“演示数据 / 固定示例，未检测本机”在各页保持可见。

偏好仍使用 [既有 PreferenceProvider](../src/app/PreferenceProvider.tsx) 与 `nexus.preferences.v1` WebView localStorage，仅持久化主题和语言；支持 Light / Dark / Follow System 与 zh-CN / en-US / 跟随系统语言，存储失败仍可继续使用当前会话并显示反馈。路由滚动复位、标题焦点、浏览器历史、筛选、空状态与错误视图保持既有行为。

[原生桥接](../src/app/native.ts) 仍仅调用 `app_metadata` 和 `apply_app_preferences`；[main capability](../src-tauri/capabilities/main.json) 保持主窗口、两项明确权限，Rust 继续负责原生菜单与窗口偏好的校验。未新增权限、通用命令执行、文件写入、网络读取、Agent discovery、凭据处理、Health 检测、Repair 或数据库行为。Model、Provider、Model Profile 与 Assignment 没有合并。

## 6. 可访问性修正与评审结论

保留 skip link、路由标题焦点、原生 select、具名导航、图标加文字的状态、页签 Arrow / Home / End 键、可见焦点、reduced motion 与 forced-colors 样式。

初次独立评审发现两项源代码 / 原生 AX 缺陷，虽当时自动检查已通过，仍进行了必要修正：

| 项目 | 最终实现与验收 |
| --- | --- |
| R1：命名选择按钮后行元数据不可访问 | 每行保留简洁选择名称、`aria-pressed`、`aria-controls`，以 `aria-describedby` 关联按钮外的本地化 `sr-only` 静态文本。描述包括 Agent、就绪、厂商 / 安装、生效 Model、分配来源。最终英 / 中原生 AX 均暴露全部四 Agent 的完整描述；双语回归逐行检查 description。 |
| R2：CTA 的可见文字与无障碍名称不一致 | 非紧凑 DetailLink 从实际可见标签和 Agent 名构造名称，紧凑图标链接保留合适的查看 / 问题名称。最终原生 AX 为 `Agent overview — Claude Code` / `查看 Agent 概览：Claude Code`；双语回归覆盖主操作、实际 Overview 导航及 Agents 页四个详情链接。 |

证据：[英文原生 AX](../.impeccable/review/phase1-ui/native/home-en-US-dark-normal.ax.txt)、[中文原生 AX](../.impeccable/review/phase1-ui/native/home-zh-CN-light-normal.ax.txt)、[回归断言](../tests/ui/production-ui.spec.ts)。原生 AX 序列化没有单独输出按钮 description 属性，因此结论是完整静态文本与关联结构可访问，不宣称已听到 VoiceOver 播报。

[finish-review.md](../.impeccable/review/phase1-ui/finish-review.md) 保留早期 `fix` 历史，并在最终 bounded verdict 中将 R1/R2 记为 resolved、限定 `ship`。最终复核只评分这两项，没有重新做全界面缺陷扫描。更广的 WCAG 一致性、实际读屏与用户审美仍待验证。

设计 detector 有两条样式建议：继承的 Notice 2px 语义边，以及参考明确采用的选中行 3px 边。它们分别用于通知和选择状态，按已有 / 用户指定参考保留；见 [detector.json](../.impeccable/review/phase1-ui/detector.json)。没有把这两条建议扩张为新视觉方向。

## 7. 基线与暂定参考比较

原始 HEAD 的前端通过临时只读副本捕获，没有创建 worktree；[baseline-source.json](../.impeccable/review/phase1-ui/baseline-source.json) 记录来源。保留 8 张原始 Home / Claude Overview × zh-CN / en-US × Light / Dark 的 1440px 浏览器 PNG。这些是原始前端比较证据，不是原始原生窗口截图。

| 维度 | 实际变化与保留的取舍 |
| --- | --- |
| 层级 | 原始 Home 使用页标题、额外就绪块、表格和下方独立问题列表；现在以就绪句进入四 Agent 与当前解释，减少重复跨区查找。 |
| 身份 | 原蓝色 N 方块 / Project Nexus 文字替换为 Joined Measure 的紧凑中断 N、小写光学字标与低彩度交互色。识别度和品牌偏好仍需要用户判断。 |
| 排版 | 原系统字体表格转换为受控 Latin / Han / Mono 分工、18px Agent 名和稳定的 13–14px 工作文字；没有靠缩小主要信息获取密度。 |
| 交互 | 采纳 Continuity 的选中行 → 上下文关系；四 Agent 状态始终在列表内，当前解释可切换，详情是独立入口。 |
| 配置 / Health | Detail 强调有效 Model、独立修订记录、局部示例检查、来源与下一步；线上可用性未检查仍明确显示。 |
| 主题 / 一致性 | Joined Measure 提供整套独立 Light / Dark 关系；没有照搬 Continuity 的蓝紫色或原型弹窗 / 研究工具栏。 |
| 桌面实用性 | 真正的路由、原生窗口、持久偏好与五个 Detail 区域可操作；上下文较长的纵向成本尚未解决。 |

对照入口：[原始 Home](../.impeccable/review/phase1-ui/baseline/home-zh-CN-light-1440.png)、[原始 Detail](../.impeccable/review/phase1-ui/baseline/agent-claude-en-US-dark-1440.png)、[Joined Measure 桌面](../design-lab/design-lock/screenshots/desktop-light-zh-CN-1440x900.png)、[Joined Measure 品牌](../design-lab/design-lock/screenshots/brand-specimen.png)、[Continuity Home](../design-lab/visual-breakthrough/screenshots/home-light-zh-CN-1440x900.png)、[当前生产 Home](../.impeccable/review/phase1-ui/browser/home-zh-CN-light-1440x900.png)、[当前原生 Detail](../.impeccable/review/phase1-ui/native/agent-claude-en-US-dark-normal.jpg)。

最终同路径重捕获的 20 张原生图与 16 张构建前端图均在实施线程中逐张打开检查。独立初次评审检查了完整原始证据矩阵；最终限定复核检查了 R1/R2 的源码、原生 AX 与四张相关当前截图。比较支持“生产 UI 有实质升级且可供用户评审”，不支持“达到最终审美标准”。

## 8. 已执行检查与真实限制

以下来自实施线程当前证据，不是文档阶段重新执行的测试。命令与新鲜度见 [validation.json](../.impeccable/review/phase1-ui/validation.json)。

| 命令 / 检查 | 结果与范围 |
| --- | --- |
| `pnpm typecheck` | PASS；R1/R2 最终前端修正后重新执行 |
| `pnpm exec eslint src tests vite.config.ts playwright.config.ts` | PASS；生产 / 测试范围，最终修正后重新执行 |
| `pnpm lint` | **FAIL**；258 errors，25 个未修改的 design-lab 文件，涉及 design-lock、final-direction、typography-craft、visual-breakthrough；[完整日志](../.impeccable/review/phase1-ui/full-lint.log) |
| `pnpm test` | PASS；4 文件、15 个单元测试，包括既有偏好 / 契约及本批次字体 / 对比度验证 |
| `pnpm test:ui` | PASS；28 个 Playwright 测试，最终修正后重新执行，日志结尾 `28 passed (3.2m)`；[日志](../.impeccable/review/phase1-ui/playwright.log)、[last-run](../test-results/.last-run.json) |
| `pnpm build` | PASS；最终桌面 build 的 `beforeBuildCommand` 再执行 TypeScript / Vite 生产构建 |
| `pnpm desktop:build --bundles app -- --locked` | PASS；最终前端状态重新构建 macOS `.app` |
| `pnpm native:check` | PASS；等价于锁定的 `cargo check`，在最终前端修正前执行；原生源码未变化，结果仍适用 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | PASS；复用同一未变化原生源码的结果 |
| `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | PASS；复用同一未变化原生源码的结果 |
| `pnpm native:test` | PASS；3 个 Rust 测试，复用同一未变化原生源码的结果 |
| 保留范围 `git diff --exit-code HEAD -- …` | PASS；fixture、src/app、Rust / Tauri、package / lock、Vite / Playwright 配置、design-lab、AGENTS 与原规范无差异；分支 / HEAD 未变化 |

Playwright 继承矩阵为 zh-CN / en-US × Light / Dark × **960×640 与 1440×1000**，覆盖七个主 / Detail 路由、既有子视图、axe、无横向溢出、键盘 / 历史 / 空错误状态、主题与语言偏好、mocked native locale / 存储失败、双语扩展标签、320px reflow 和 reduced motion。新增回归覆盖选择与配置真实语义、R1/R2、动作焦点与本地字体。320px reflow 是浏览器近似检查；mocked native 失败路径也不是原生 IPC 的现场故障验证。

独立构建前端截图矩阵另外使用 **1440×900 与 960×640**，不能与 Playwright 的 1440×1000 混为同一截图尺寸。对比度单元测试要求已列文本 / 状态组合 ≥4.5:1，控制 / 焦点 ≥3:1；通过这些组合与 axe 不等于完整 WCAG 2.2 AA 认证。

28 项 UI 测试已全部完成后，测试 web-server 的遗留 Vite 子进程才被停止，runner 随后退出 0，`.last-run.json` 为 `passed` 且无失败测试。最终没有 Vite / preview 服务残留；正在运行的是打包应用，`tauri://localhost` 加载本地资源。

## 9. 原生与构建前端证据

### 当前包与主机

最终应用路径为 `<repository root>/src-tauri/target/release/bundle/macos/Project Nexus.app`。记录的文件总大小为 19,102,224 字节（18.22 MiB）。主可执行文件 SHA-256 为 `b242b9a0f6051b501774c484422d0fe54f94e4f031e1cdf3b21b08d24042199a`；这是可执行文件指纹，不是整个目录的单一签名。

观测主机为 macOS 27.2 / arm64，见 [delivery-state.json](../.impeccable/review/phase1-ui/delivery-state.json)。这些是本次测试主机事实，不是 Nexus 支持最低 OS 的声明，也不改变页面里的固定 System fixture。

交付时应用停留在 Home、zh-CN、Follow System（当前解析为 Dark）。原生最终窗口为逻辑 **1200×800**，截图为 **2400×1600**。原生标题栏和内容滚动、主题 / 语言控件、选择 / 展开、键盘焦点已实际观察。紫色共享标记和指针光晕是 macOS 自动化叠层，不是 Nexus 设计元素。

### 原生正常窗口矩阵

共 8 张 Home / Claude Overview 矩阵 JPG；每张有同名 `.ax.txt`。另 12 张动作 / 最终状态图构成 20 张最终原生截图。清单见 [native/manifest.json](../.impeccable/review/phase1-ui/native/manifest.json)，截图均为实际 JPEG。

| 语言 / 主题 | Home | Claude Overview |
| --- | --- | --- |
| zh-CN / Light | [原生 JPG](../.impeccable/review/phase1-ui/native/home-zh-CN-light-normal.jpg) | [原生 JPG](../.impeccable/review/phase1-ui/native/agent-claude-zh-CN-light-normal.jpg) |
| zh-CN / Dark | [原生 JPG](../.impeccable/review/phase1-ui/native/home-zh-CN-dark-normal.jpg) | [原生 JPG](../.impeccable/review/phase1-ui/native/agent-claude-zh-CN-dark-normal.jpg) |
| en-US / Light | [原生 JPG](../.impeccable/review/phase1-ui/native/home-en-US-light-normal.jpg) | [原生 JPG](../.impeccable/review/phase1-ui/native/agent-claude-en-US-light-normal.jpg) |
| en-US / Dark | [原生 JPG](../.impeccable/review/phase1-ui/native/home-en-US-dark-normal.jpg) | [原生 JPG](../.impeccable/review/phase1-ui/native/agent-claude-en-US-dark-normal.jpg) |

| 实际操作 / 状态 | 最终原生证据 |
| --- | --- |
| Home 键盘焦点、Gemini 选择及健康依据展开 | [焦点](../.impeccable/review/phase1-ui/native/home-en-US-dark-keyboard-focus.jpg)、[选择](../.impeccable/review/phase1-ui/native/home-gemini-en-US-dark-selection.jpg)、[依据](../.impeccable/review/phase1-ui/native/home-gemini-en-US-dark-evidence.jpg) |
| Home 纵向滚动查看 System / 活动 | [下部滚动](../.impeccable/review/phase1-ui/native/home-en-US-dark-lower-scroll.jpg) |
| Overview 操作打开 Model、选中页签键盘焦点 | [Model 动作](../.impeccable/review/phase1-ui/native/agent-claude-en-US-dark-model-action.jpg)、[Model 焦点](../.impeccable/review/phase1-ui/native/agent-claude-en-US-dark-model-keyboard.jpg) |
| 其余三个 Detail 页签 | [Diagnostics](../.impeccable/review/phase1-ui/native/agent-claude-en-US-dark-diagnostics.jpg)、[Configuration](../.impeccable/review/phase1-ui/native/agent-claude-en-US-dark-configuration.jpg)、[Extensions](../.impeccable/review/phase1-ui/native/agent-claude-en-US-dark-extensions.jpg) |
| 导航折叠后偏好仍可用；重新启动后语言 / 主题保留 | [折叠](../.impeccable/review/phase1-ui/native/home-en-US-dark-collapsed.jpg)、[重启](../.impeccable/review/phase1-ui/native/home-en-US-dark-after-relaunch.jpg) |
| 最终中文 Home、Follow System 当前 Dark | [交付状态](../.impeccable/review/phase1-ui/native/home-zh-CN-system-review.jpg) |

**原生 960×640 未完成验证。** 边 / 角拖动没有可靠建立最终所需尺寸，最终捕获仍为 1200×800；错误的暂定尺寸标记已弃用。`native-round1` 的早期较短窗口也不是最终窄窗证据。不能以截图存在、配置允许缩放或下面的 Chrome 结果宣称原生窄窗通过。

### 构建前端矩阵

16 张最终 PNG 为 Home / Claude Overview × 两语言 × 两主题 × 1440×900 / 960×640；逐项 `mainOverflow: false`，见 [browser/manifest.json](../.impeccable/review/phase1-ui/browser/manifest.json)。它们验证构建前端的浏览器几何与渲染，不能替代 Tauri native WebView 窄窗验证。

| 语言 / 主题 | 1440×900 Home / Detail | 960×640 Home / Detail |
| --- | --- | --- |
| zh-CN / Light | [Home](../.impeccable/review/phase1-ui/browser/home-zh-CN-light-1440x900.png) / [Detail](../.impeccable/review/phase1-ui/browser/agent-claude-zh-CN-light-1440x900.png) | [Home](../.impeccable/review/phase1-ui/browser/home-zh-CN-light-960x640.png) / [Detail](../.impeccable/review/phase1-ui/browser/agent-claude-zh-CN-light-960x640.png) |
| zh-CN / Dark | [Home](../.impeccable/review/phase1-ui/browser/home-zh-CN-dark-1440x900.png) / [Detail](../.impeccable/review/phase1-ui/browser/agent-claude-zh-CN-dark-1440x900.png) | [Home](../.impeccable/review/phase1-ui/browser/home-zh-CN-dark-960x640.png) / [Detail](../.impeccable/review/phase1-ui/browser/agent-claude-zh-CN-dark-960x640.png) |
| en-US / Light | [Home](../.impeccable/review/phase1-ui/browser/home-en-US-light-1440x900.png) / [Detail](../.impeccable/review/phase1-ui/browser/agent-claude-en-US-light-1440x900.png) | [Home](../.impeccable/review/phase1-ui/browser/home-en-US-light-960x640.png) / [Detail](../.impeccable/review/phase1-ui/browser/agent-claude-en-US-light-960x640.png) |
| en-US / Dark | [Home](../.impeccable/review/phase1-ui/browser/home-en-US-dark-1440x900.png) / [Detail](../.impeccable/review/phase1-ui/browser/agent-claude-en-US-dark-1440x900.png) | [Home](../.impeccable/review/phase1-ui/browser/home-en-US-dark-960x640.png) / [Detail](../.impeccable/review/phase1-ui/browser/agent-claude-en-US-dark-960x640.png) |

Windows build / runtime、macOS Intel、VoiceOver / NVDA / Narrator 实际语音、签名 / 公证、OS zoom、实时原生 OS 主题变化均未验证。原生选择 Follow System 后解析为当前 Dark 已观察；运行中更改 OS 主题的现场结果仍未知。

## 10. 启动真实桌面应用

在项目根目录运行开发桌面应用：

```sh
pnpm desktop:dev
```

或直接打开已存在的最终包（无需 Vite 服务）：

```sh
open "<repository root>/src-tauri/target/release/bundle/macos/Project Nexus.app"
```

本批次实际使用的桌面构建命令为：

```sh
pnpm desktop:build --bundles app -- --locked
```

评审对象是运行中的 Project Nexus：查看 Home 与 Claude Detail，切换两语言和 Light / Dark，选择 Gemini、展开健康依据、滚动、折叠导航，并用键盘触发 Overview 的 Model 动作。现有包无需网络获取字体或业务数据。

## 11. 已知视觉弱点与建议 B-2 范围

Home 的解释列决定共享工作面高度，四行 Agent 下方仍有明显空白；展开健康依据会进一步增高。System 和活动位于首屏下方，英文 960×640 在到达主操作前需要更多滚动。没有内容截断不代表密度已达到理想状态。

窄 Detail 的健康依据展开项位于 Important Extensions 之后；需要用户判断它是否应更早出现在问题 / 有效配置附近。字体双语比例在现有截图可用，但品牌克制程度、Source Han 中文字形与长时间舒适度仍是审美 / 使用判断；本轮没有独立语言使用研究或品牌识别 / 商标结论。约 7.97 MB 字体成本也保留为未来优化问题，当前没有擅自子集化。

**Phase 1.1B-2 仅为建议，尚未授权。** 用户明确评审本次真实应用后，建议继续限定为现有 Home / Agent Detail 与必要共享 UI：

1. 结合用户反馈压紧 Home 重复解释和证据分组，降低列表下方空白与窄窗动作前的滚动成本；保留 13–14px 阅读文字、所有状态与真实性说明。
2. 调整窄 Detail 健康依据的优先顺序，验证它与问题 / 生效配置 / 下一步的关系；不扩张为新 Diagnostics / Repair 工作流。
3. 补齐最终原生 960×640、Windows 字体 / 标题栏 / 键盘、实际读屏、OS zoom 与运行中 OS 主题变化证据； macOS Intel 等平台缺口继续明确保留，签名 / 公证仍是独立分发验证。
4. 按运行应用的明确反馈校准中文 / Latin 密度与局部组件；若要将暂定视觉纳入正式规范、优化字体打包或扩展到其余页面，先获得对应授权。

本批次到此停止。**Phase 2 未启动，未创建 Git commit。等待用户明确评审运行中的桌面应用，再决定下一批 UI 工作。**
