# PRD v0.5：剧造 PlotForge

> 日期：2026-06-07  
> 状态：Final Draft / 命名统一版 / 可作为仓库初版 PRD  
> 开发方式：单人开发 + Codex 辅助  
> 推荐技术路线：Rust Core + Tauri Desktop + React/TypeScript Editor  
> 本版重点：统一品牌名为「剧造 PlotForge」，补强剧情生产体系、Steam 产品化路线、UGC/Workshop 发布策略、AI 内容合规策略。

---

## 目录

1. [最终 Review 结论](#1-最终-review-结论)
2. [产品定位](#2-产品定位)
3. [核心战略判断](#3-核心战略判断)
4. [参考案例与可借鉴设计](#4-参考案例与可借鉴设计)
5. [目标用户](#5-目标用户)
6. [产品目标与成功标准](#6-产品目标与成功标准)
7. [产品范围](#7-产品范围)
8. [核心设计原则](#8-核心设计原则)
9. [核心概念模型](#9-核心概念模型)
10. [剧情生产系统：Story Craft Pipeline](#10-剧情生产系统story-craft-pipeline)
11. [运行时架构](#11-运行时架构)
12. [多 Agent 设计](#12-多-agent-设计)
13. [规则与世界推演系统](#13-规则与世界推演系统)
14. [多模态媒体管线](#14-多模态媒体管线)
15. [创作者工作台](#15-创作者工作台)
16. [高级开发者模式](#16-高级开发者模式)
17. [Steam 产品化与发布路线](#17-steam-产品化与发布路线)
18. [桌面端技术架构](#18-桌面端技术架构)
19. [项目工程格式](#19-项目工程格式)
20. [Provider 与模型接入](#20-provider-与模型接入)
21. [调试、可观测性与成本控制](#21-调试可观测性与成本控制)
22. [导出与发布策略](#22-导出与发布策略)
23. [安全、权限、版权与合规](#23-安全权限版权与合规)
24. [MVP 功能需求](#24-mvp-功能需求)
25. [非功能需求](#25-非功能需求)
26. [单人 + Codex 开发路线图](#26-单人--codex-开发路线图)
27. [首个内置 Demo：王朝余烬](#27-首个内置-demo王朝余烬)
28. [风险与应对](#28-风险与应对)
29. [待决问题](#29-待决问题)
30. [附录：建议的首批 Codex 任务](#30-附录建议的首批-codex-任务)
31. [参考资料](#31-参考资料)

---

# 1. 最终 Review 结论

## 1.1 总体结论

本 PRD 的主方向成立，可以作为项目启动文档。推荐路线保持不变：

```text
桌面端 PlotForge Studio
+ Rust 核心引擎
+ Tauri 桌面壳
+ React/TypeScript 编辑器 UI
+ Story / Scene / Beat Runtime
+ WorldState / Rule Engine
+ 多 Agent 内容生产流水线
+ 图片 / 声音多模态管线
+ 本地工程文件 + 可导出运行时
```

本轮 Review 后，发现并补齐两个关键缺口：

1. **剧情生产体系不足**：图文游戏的核心不是“AI 能生成文字”，而是“剧情能持续好看”。需要引入类似网络小说工业化写作中的钩子、爽点/情绪满足、伏笔、反转、人物弧线、文风、审稿与一致性检查机制。
2. **Steam 路线需要合规重构**：把产品做成可以上架 Steam 的“创作型游戏”是可行方向，但不能承诺玩家一键把自己的游戏上架 Steam 商店。更合理的是：Steam 游戏本体内置创作与游玩，玩家作品先发布到 Steam Workshop；高级创作者可导出独立包，再自行走 Steam Direct 上架流程。

## 1.2 本版新增/修正重点

| 领域 | v0.3 状态 | v0.4 修正 |
|---|---|---|
| 剧情系统 | 有 StoryState，但偏状态记忆 | 新增 Story Craft Pipeline：钩子、情绪曲线、伏笔、反转、人物弧线、剧情审稿 |
| 参考案例 | InfiPlot / 崇祯 | 新增 oh-story-claudecode 作为剧情工业化参考 |
| Steam 路线 | 仅讨论 Web/桌面导出 | 新增 Steam 产品化策略：Steam 游戏本体 + Workshop UGC + 导出 Steam Submission Kit |
| AI 合规 | 说明 API Key 和内容安全 | 增加 Steam AI 内容披露、Live-generated guardrails、UGC 审核策略 |
| 版权风险 | 未充分展开 | 明确扫榜/拆文/参考库默认不抓取版权正文；只允许用户授权导入或公开许可资料 |
| MVP 范围 | 偏“引擎” | 收窄为“一个强剧情历史危机模拟 Demo + 创作工作台骨架” |

## 1.3 最终建议

产品应采用双重定位：

```text
开发者视角：PlotForge Engine / 剧造引擎，一套 AI 图文互动游戏创作引擎
玩家视角：PlotForge: AI Story Game Maker / 剧造：游戏工坊，一款可以在其中创造、游玩、分享 AI 图文游戏的 Steam 创作型游戏
```

第一阶段不要急着做“开放平台”。先做一个能在本地跑通、能生成好剧情、能调试状态、能导出静态体验的桌面工具。Steam 路线放入中期里程碑，用 Workshop 承接玩家 UGC。

---

# 2. 产品定位

## 2.1 一句话定位

> **剧造 PlotForge：把一个想法，变成一款可玩的 AI 图文游戏。**

## 2.2 Steam 化的一句话定位

> **PlotForge: AI Story Game Maker / 剧造：游戏工坊** 是一款上架 Steam 的 AI 创作型游戏：玩家在游戏里创造自己的图文互动游戏、试玩、迭代、分享，并通过 Workshop 订阅、发布和 remix 其他玩家的作品。

## 2.3 品牌命名体系

| 层级 | 名称 | 说明 |
|---|---|---|
| 总品牌 | **剧造 PlotForge** | 中文强调“剧情创造 / 剧场创造”，英文强调“锻造剧情与游戏” |
| 桌面创作工具 | **PlotForge Studio / 剧造工坊** | MVP 主产品形态，面向认真创作者和高级开发者 |
| 核心引擎 | **PlotForge Engine / 剧造引擎** | Rust Core + Runtime + Rule + Agent Pipeline |
| 玩家运行时 | **PlotForge Player** | 本地预览、Web 导出、桌面运行包 |
| Steam 版本 | **PlotForge: AI Story Game Maker / 剧造：游戏工坊** | 面向玩家的“制作游戏的游戏” |
| UGC / Workshop | **PlotForge Workshop** | 玩家作品分享、订阅、评分、更新 |
| 导出助手 | **PlotForge Export Kit** | 高级创作者的 Web / Desktop / Steam-ready 打包工具 |
| 工程扩展名 | `.plotforge` / `.pfproj` | 项目清单或打包文件扩展名 |
| CLI | `plotforge` | 开发者命令行工具 |

对外短文案：

```text
中文：剧造：把一个想法，变成一款 AI 图文游戏。
英文：PlotForge turns your ideas into playable AI story games.
Steam：剧造是一款可以创造游戏的游戏。
```

## 2.4 产品本质

剧造 PlotForge 不是传统 3D 游戏引擎，也不是 AI 聊天壳，而是：

```text
AI 互动叙事运行时
+ AI 世界状态推演引擎
+ AI 剧情工业化系统
+ 多 Agent 内容生产流水线
+ 图像/声音生成管线
+ 创作者桌面工作台
+ 高级开发者工程系统
+ 玩家运行时导出系统
+ Steam UGC 分享系统
```

类比：

```text
RPG Maker
+ Twine
+ 橙光编辑器
+ Ren'Py
+ InfiPlot
+ AI Dungeon
+ 历史模拟器：崇祯
+ 网文写作工作流
+ Steam Workshop UGC 游戏
```

## 2.5 当前阶段不做什么

当前阶段不做：

```text
通用 3D 引擎
实时动作游戏引擎
大型多人在线 AI 世界
官方云端创作者社区
素材市场
复杂视频生成平台
纯聊天角色平台
玩家一键代上架 Steam 商店
```

---

# 3. 核心战略判断

## 3.1 桌面端优先

建议继续采用：

```text
PlotForge Studio：桌面端优先
PlotForge Engine：Rust
Desktop Shell：Tauri
Editor UI：React + TypeScript
PlotForge Player：Web 优先，可本地/桌面导出
PlotForge: AI Story Game Maker：中期目标，可包装为创作型游戏
```

原因：创作者需要本地文件、Git、VS Code、Codex、模型 API Key、本地缓存、素材管理、调试器和离线编辑。桌面端天然更适合创作工具。

## 3.2 Rust 合适，但 UI 不要纯 Rust

Rust 适合承担核心引擎：

- 强类型数据模型。
- 状态机、规则系统、存档系统可靠。
- Provider、任务队列、缓存、导出能力可复用。
- crate 结构对 Codex 友好。

但编辑器 UI 复杂，推荐 Web UI：

```text
Rust：engine / runtime / rules / storage / media / export
TypeScript：editor UI / forms / graph / preview
Tauri：bridge / permissions / packaging
```

## 3.3 “创作平台”与“Steam 游戏”不冲突

不要把二者视为二选一。推荐双层产品：

| 层级 | 形态 | 目标 |
|---|---|---|
| PlotForge Studio / 剧造工坊 | 桌面创作工具 | 给认真创作者和高级开发者使用 |
| PlotForge: AI Story Game Maker / 剧造：游戏工坊 | 创作型游戏包装 | 给玩家化创作者使用，强调创造、游玩、分享、挑战 |
| PlotForge Player | Web/桌面运行时 | 给玩家游玩创作者作品 |
| PlotForge Workshop | Steam Workshop | 承接玩家作品发布、订阅、remix |

---

# 4. 参考案例与可借鉴设计

## 4.1 InfiPlot

InfiPlot 最值得借鉴的是 Story → Scene → Beat 的结构，以及多 Agent 内容生产流水线。

可吸收点：

- Scene 是视觉单位。
- Beat 是交互单位。
- Writer 两阶段：先规划 Scene，再写完整 Beats。
- Agent 拆成 Architect / Writer / CharacterDesigner / Cinematographer / Painter。
- sceneKey 与角色肖像 reference 用于视觉连续性。
- Prefetch 用于降低切场等待。

我们不能只停留在视觉小说，还要补上 WorldState、Rule Engine、事件系统与创作者工具。

## 4.2 历史模拟器：崇祯

可吸收点：

- 自然语言作为策略输入。
- AI 推演国家状态变化。
- 朝臣/派系提供不同立场反馈。
- 玩家决策必须有世界后果。

我们需要把“自然语言 → 结构化行动 → 规则计算 → 世界状态变化 → 剧情表现”做成引擎能力。

## 4.3 oh-story-claudecode

oh-story-claudecode 是一个 AI 网文写作 skill 包，覆盖扫榜、拆文、写作、去 AI 味、封面图等流程。它的核心思路是“套路 = 确定性的情绪满足”，并把专业作者方法论拆成扫榜、拆文、商业化写作三步，同时围绕爆款逆向、剧情模块化重组、上下文状态分层管理、人机协同展开。

可吸收点：

- **剧情不是自由生成，而是可工程化拆解。**
- **剧情模块化**：钩子、爽点、反转、伏笔、情绪曲线、人物弧线。
- **分层文件系统**：设定、大纲、正文、追踪、参考资料分开管理。
- **多 Agent 审稿**：架构、角色、叙事、一致性、研究、探索、章节提取分工。
- **上下文恢复机制**：压缩前保存快照，压缩后恢复关键上下文。
- **去 AI 味**：把 AI 输出作为初稿，必须经过文风与叙事质量精修。

对我们的映射：

| oh-story 模块 | 游戏引擎中的映射 |
|---|---|
| 扫榜 | 题材/模板研究，P1 后做，不抓版权正文 |
| 拆文 | 参考作品结构分析，P1 后做，仅用户授权输入 |
| 大纲 | Story Arc / Campaign Arc |
| 细纲 | Scene Plan / Quest Plan |
| 正文 | Beat Script |
| 伏笔.md | PlotThread Registry |
| 时间线.md | Game Timeline |
| 角色状态.md | CharacterState / RelationshipState |
| 文风.md | StyleGuide / ToneGuide |
| story-review | Narrative QA / Plot Doctor |
| story-explorer | 只读剧情查询 Agent |
| consistency-checker | Canon / State / Continuity Validator |

## 4.4 Steam Workshop 与 UGC 游戏

Steam Workshop 适合作为玩家作品发布和订阅渠道。Steamworks 文档明确说 Workshop 是玩家和社区成员参与为游戏创建内容的地方，并提供 ready-to-use 与 curated 两种主要集成模式。

我们的适配：

```text
玩家作品 = Workshop item
作品格式 = .aigame 或项目压缩包
订阅 = 下载到本地作品库
游玩 = Player Runtime 加载作品
remix = 复制为本地工程
上传 = 通过 Workshop UGC API
```

不建议一开始做付费 Workshop 内容；先做免费分享和订阅。

---

# 5. 目标用户

## 5.1 Persona A：非程序创作者

### 特征

- 会写故事、设定、角色。
- 不会或不想写复杂代码。
- 想做 AI 互动小说、恋爱游戏、宫斗游戏、推理游戏、历史模拟游戏。

### 主要诉求

- 快速创建世界观。
- 创建角色与关系。
- 让 AI 帮忙扩展剧情。
- 生成场景图和角色图。
- 分享给别人玩。

## 5.2 Persona B：高级创作者 / 独立开发者

### 特征

- 会写代码。
- 想控制规则、事件、数值、Agent、Prompt。
- 会用 Git、VS Code、Codex、命令行。

### 主要诉求

- 自定义工程结构。
- 自定义状态变量和规则。
- 自定义 Agent Prompt。
- 接入自己的模型 Provider。
- 导出可发布版本。

## 5.3 Persona C：玩家化创作者

### 特征

- 在 Steam 上购买游戏。
- 不一定认为自己是开发者。
- 喜欢“创造自己的故事世界”。
- 喜欢订阅、游玩、remix 别人的作品。

### 主要诉求

- 游戏内直接创建一个作品。
- 有模板和挑战。
- 能发布到 Workshop。
- 能被别人游玩和点赞。

## 5.4 Persona D：普通玩家

### 特征

- 只玩作品。
- 希望体验流畅。
- 不想看到复杂编辑器。

### 主要诉求

- 快速进入游戏。
- 剧情好看。
- 图片/声音稳定。
- 选择有后果。
- 存档可靠。

---

# 6. 产品目标与成功标准

## 6.1 MVP 目标

> 创作者可以在桌面端创建一个 AI 图文互动游戏项目，配置世界、角色、状态和规则，通过多 Agent 生成剧情、场景和图像，在本地试玩，并导出一个静态 Web 体验包。

## 6.2 v1.0 目标

> 产品可作为创作型游戏上架 Steam，玩家可以在游戏中创建 AI 图文游戏、试玩、发布到 Steam Workshop，并订阅/游玩他人作品。

## 6.3 成功标准

### 创作者侧

- 30 分钟内创建一个可玩的 Demo。
- 能配置至少 5 个角色。
- 能配置至少 5 个世界状态变量。
- 能创建至少 3 个事件规则。
- 能运行本地试玩。
- 能查看每次 AI 生成和状态变化。
- 能导出静态 Web 包。

### 剧情质量侧

- 每个 Scene 有明确戏剧目的。
- 每个 Scene 至少有一个钩子、推进或情绪回报。
- 主线伏笔可被追踪。
- 角色关系和状态不会无故重置。
- 剧情审稿器能标记至少 4 类问题：无推进、角色 OOC、伏笔断线、状态冲突。

### 玩家侧

- 每个 Scene 可看到图文内容。
- 玩家选择能影响后续剧情或状态。
- 普通 Scene 切换不崩溃。
- 存档可恢复。

### 开发侧

- 核心引擎可用 CLI 跑通。
- 所有 schema 可测试。
- Agent 可 mock。
- `cargo check --workspace` 可通过。
- Codex 可以按 crate 独立执行任务。

---

# 7. 产品范围

## 7.1 MVP 必做

| 模块 | 优先级 |
|---|---:|
| Rust workspace + CLI prototype | P0 |
| 桌面端工程管理 | P0 |
| 项目文件夹格式 | P0 |
| 世界设定编辑 | P0 |
| 角色卡编辑 | P0 |
| 状态变量编辑 | P0 |
| 声明式规则编辑 | P0 |
| Story / Scene / Beat Runtime | P0 |
| Story Craft 基础：钩子/伏笔/情绪曲线字段 | P0 |
| 多 Agent 生成流水线 | P0 |
| 剧情审稿 / 一致性检查 | P0 |
| 图片生成接入 | P0 |
| 本地试玩 | P0 |
| 调试面板 | P0 |
| 存档 | P0 |
| 静态 Web 导出 | P1 |
| TTS 接入 | P1 |
| Steam Workshop | P2 |

## 7.2 MVP 不做

```text
多人协作
官方云托管
素材市场
多模板大而全
复杂节点编辑器
任意脚本插件
Steam Workshop 集成
动态 Web 托管
玩家一键上架 Steam 商店
```

## 7.3 范围控制原则

第一阶段只围绕一个强验证 Demo：

```text
《王朝余烬》：历史危机模拟 + 剧情驱动 + 世界状态推演
```

所有架构都服务于这个 Demo 跑通。

---

# 8. 核心设计原则

## 8.1 剧情是灵魂

图文游戏的核心体验不是“生成了多少文本”，而是剧情是否持续有吸引力。

必须把剧情拆成可管理对象：

```text
题材承诺
玩家期待
开场钩子
情绪曲线
爽点/回报
伏笔
反转
人物弧线
章节/场景目的
节奏密度
结尾扣子
```

## 8.2 Scene 是视觉单位

图片生成以 Scene 为单位，而不是每个 Beat 都生成图。

## 8.3 Beat 是交互单位

玩家阅读、点击继续、选择、自由输入都发生在 Beat 层。

## 8.4 WorldState 是游戏事实

WorldState 记录资源、时间、地点、势力、角色状态、事件状态。它是游戏事实来源。

## 8.5 StoryState 是叙事连续性

StoryState 记录当前梗概、人物关系、伏笔、下一个钩子。

## 8.6 StoryCraftState 是剧情质量状态

StoryCraftState 记录情绪曲线、剧情承诺、伏笔回收、角色弧线、节奏评分、风险点。

## 8.7 LLM proposes, Engine commits

LLM 可以提出剧情、状态变化和媒体请求，但最终提交必须经过规则系统和 Validator。

## 8.8 媒体生成必须成本可控

图片、语音、Prefetch 都必须缓存、复用、可取消、可预算控制。

## 8.9 文件工程是源数据

文件工程是 source of truth，SQLite 是缓存、索引、trace、资产数据库。

## 8.10 Steam 路线必须遵守平台现实

玩家作品优先进入 Workshop；独立上架 Steam 商店必须由作品作者自己走 Steam Direct、Steamworks、内容审核和发布流程。

---

# 9. 核心概念模型

## 9.1 GameProject

```ts
type GameProject = {
  id: string;
  title: string;
  description: string;
  engineVersion: string;

  worldBible: WorldBible;
  storyBible: StoryBible;
  storyCraftBible: StoryCraftBible;
  visualBible: VisualBible;
  audioBible: AudioBible;
  ruleBible: RuleBible;

  characters: Character[];
  locations: Location[];
  events: EventDefinition[];
  resources: ResourceDefinition[];
  agents: AgentDefinition[];
  exportProfiles: ExportProfile[];
};
```

## 9.2 StoryCraftBible

```ts
type StoryCraftBible = {
  targetAudience?: string;
  genrePromise: string[];
  emotionalContract: string[];
  pacingProfile: PacingProfile;
  hookStrategy: HookStrategy;
  reversalStrategy?: ReversalStrategy;
  proseStyleGuide?: string;
  bannedCliches?: string[];
  referenceModules?: ReferenceModule[];
};
```

## 9.3 WorldState

```ts
type WorldState = {
  time: GameTime;
  resources: Record<string, number>;
  flags: Record<string, boolean>;
  factions: Record<string, FactionState>;
  characters: Record<string, CharacterState>;
  locations: Record<string, LocationState>;
  activeEvents: EventState[];
  history: WorldEventLog[];
};
```

## 9.4 StoryState

```ts
type StoryState = {
  spine: {
    logline: string;
    genreTags: string[];
    protagonist: string;
    mainQuestion: string;
    coreCastNotes: string;
  };
  dynamic: {
    synopsis: string;
    openThreads: PlotThread[];
    relationships: RelationshipState[];
    nextHook?: string;
    recentEvents: string[];
  };
};
```

## 9.5 StoryCraftState

```ts
type StoryCraftState = {
  activePromises: StoryPromise[];
  emotionalArc: EmotionalBeat[];
  plotThreads: PlotThread[];
  characterArcs: CharacterArc[];
  pacingScore?: number;
  tensionScore?: number;
  aiSlopRisk?: number;
  reviewNotes: NarrativeReviewNote[];
};
```

## 9.6 PlotThread

```ts
type PlotThread = {
  id: string;
  title: string;
  type: "mystery" | "foreshadow" | "relationship" | "political" | "survival" | "custom";
  status: "open" | "deepened" | "paid_off" | "abandoned";
  introducedAt: string;
  expectedPayoff?: string;
  relatedCharacters: string[];
  relatedWorldFlags: string[];
};
```

## 9.7 Scene

```ts
type Scene = {
  id: string;
  sceneKey: string;
  locationKey?: string;
  sceneSummary: string;
  dramaticPurpose: string;
  hook?: string;
  emotionalGoal?: string;
  worldStateSnapshotId?: string;
  storyCraftStatePatch?: StoryCraftPatch;
  imageAssetId?: string;
  beats: Beat[];
  entryBeatId: string;
};
```

## 9.8 Beat

```ts
type Beat = {
  id: string;
  narration?: string;
  speaker?: string;
  line?: string;
  lineDelivery?: string;
  activeCharacters?: ActiveCharacter[];
  narrativeFunction?: "hook" | "setup" | "payoff" | "reversal" | "choice" | "cliffhanger";
  next: BeatNext;
};
```

---

# 10. 剧情生产系统：Story Craft Pipeline

## 10.1 为什么需要独立剧情系统

图文游戏不像普通工具型 Agent，玩家会直接感受到：

```text
剧情有没有钩子
角色是否立得住
选择是否有分量
伏笔是否回收
每场是否推进
情绪是否有回报
AI 味是否明显
```

所以剧情生产必须成为 P0，而不是 Writer prompt 里的几句要求。

## 10.2 剧情生产流程

```text
Story Architect
  ↓
Story Craft Planner
  ↓
Scene Planner
  ↓
Beat Writer
  ↓
Plot Doctor
  ↓
Consistency Checker
  ↓
Deslop / Style Refiner
  ↓
Validator
```

## 10.3 Scene 级剧情约束

每个 Scene 必须具备：

```text
1. dramaticPurpose：本场戏为什么存在
2. hook：开头抓人的冲突/悬念/情绪点
3. emotionalGoal：玩家读完这一场应获得什么情绪
4. worldConsequence：是否有世界状态影响
5. plotThreadUpdate：推进/新增/回收哪些伏笔
6. exitPressure：结尾是否给出继续动力
```

## 10.4 剧情质量评分

MVP 提供文本评分，不直接阻断生成：

```ts
type NarrativeReview = {
  sceneId: string;
  hookScore: number;
  pacingScore: number;
  characterConsistencyScore: number;
  payoffScore: number;
  choiceMeaningfulnessScore: number;
  aiSlopRisk: number;
  issues: NarrativeIssue[];
};
```

问题类型：

```text
NO_DRAMATIC_PROGRESS
WEAK_HOOK
CHOICE_IS_FAKE
CHARACTER_OOC
THREAD_FORGOTTEN
WORLD_STATE_IGNORED
TOO_MUCH_EXPOSITION
AI_SLOP_STYLE
```

## 10.5 剧情参考库

参考库用于学习结构，不默认抓取版权正文。

```text
references/
  genre_patterns/
  hook_patterns/
  reversal_patterns/
  dialogue_patterns/
  pacing_templates/
  user_imports/
```

规则：

- MVP 只内置通用方法论模板，不内置具体版权作品正文。
- P1 支持用户导入自己有权使用的文本。
- 拆文输出只保存结构化分析和短摘要，不保存大段原文。
- 禁止默认平台爬取小说正文。

## 10.6 Story Explorer

只读查询 Agent，用来回答：

```text
某角色当前状态是什么？
哪些伏笔还没回收？
上一场玩家做了什么？
世界状态为什么变成这样？
这个角色为什么现在反对玩家？
```

这是创作者调试和玩家回顾都需要的能力。

---

# 11. 运行时架构

## 11.1 Runtime 主循环

```text
玩家进入 Scene
  ↓
显示背景图 + 当前 Beat
  ↓
玩家点击继续 / 选择 / 自由输入
  ↓
如果 continue：进入同 Scene 下一个 Beat
  ↓
如果 advanceBeat：进入同 Scene 指定 Beat
  ↓
如果 freeform / changeScene：
    1. 理解玩家意图
    2. Simulation Engine 计算世界变化
    3. Rule Engine 校验
    4. Story Craft Planner 规划剧情功能
    5. Agent Pipeline 生成内容
    6. Plot Doctor / Consistency Checker 审查
    7. Media Pipeline 生成/复用图片和语音
    8. 保存状态与 trace
    9. 进入下一 Scene
```

## 11.2 状态提交原则

```text
PlayerInput
  → ActionIntent
  → ProposedWorldDelta
  → RuleEngine
  → ValidatedWorldDelta
  → StoryStatePatch
  → StoryCraftStatePatch
  → Scene
  → Commit
```

LLM 不直接改 WorldState。

---

# 12. 多 Agent 设计

## 12.1 内容生产型 Agent

| Agent | 职责 |
|---|---|
| Story Architect | 总体故事架构、主线问题、核心承诺 |
| Story Craft Planner | 钩子、情绪曲线、伏笔、反转、场景目的 |
| Scene Planner | 生成 sceneSummary、sceneKey、cast、entryBeat |
| Beat Writer | 写具体旁白、对话、选择 |
| Character Designer | 角色视觉卡、声音卡、人设和语言风格 |
| Image Director | 镜头、构图、图像 prompt |
| Voice Director | lineDelivery、语气、TTS 策略 |
| Plot Doctor | 审稿、查无推进、查弱钩子、查假选择 |
| Consistency Checker | 查设定冲突、状态冲突、角色 OOC |
| Deslop Refiner | 降低 AI 味、改善文风 |

## 12.2 世界模拟型 Agent

| Agent | 职责 |
|---|---|
| Game Master | 综合解释玩家行动 |
| Rule Arbiter | 规则裁决 |
| NPC Agent | 重要 NPC 的动机与反应 |
| Faction Agent | 势力行为 |
| Event Director | 事件触发与长期危机 |
| Memory Curator | 记忆压缩与检索 |

## 12.3 Two-Phase Scene Generation

```text
Phase A：Scene Plan
  输出 sceneSummary / sceneKey / dramaticPurpose / cast / entryActiveCharacters

并行：
  A. Beat Writer 写完整 beats
  B. Character Designer 补新角色卡
  C. Image Director 写图像 prompt
  D. Story Craft Planner 更新剧情结构

Phase C：Review & Assemble
  - Plot Doctor 审稿
  - Consistency Checker 检查
  - Validator 校验 schema
  - 合并 WorldState / StoryState / StoryCraftState
  - 返回 Scene
```

## 12.4 Fallback 策略

| 失败点 | 降级策略 |
|---|---|
| Scene Plan 失败 | 使用模板场景 |
| Beat Writer 失败 | 生成单 Beat fallback |
| Plot Doctor 失败 | 不阻断，仅记录警告 |
| Image 失败 | 占位图 |
| TTS 失败 | 静音 |
| Rule 失败 | 不提交 delta，提示玩家命令无效 |

---

# 13. 规则与世界推演系统

## 13.1 核心原则

规则系统负责事实，剧情系统负责表现。

```text
规则决定：国库 +10，民心 -6
剧情表现：户部筹得银两，但地方怨声渐起
```

## 13.2 Rule Definition

MVP 只支持声明式规则，不支持任意脚本执行。

```json
{
  "id": "increase_tax",
  "name": "加税",
  "when": {
    "action.type": "increase_tax"
  },
  "effects": [
    { "path": "resources.treasury", "op": "add", "value": 10 },
    { "path": "resources.public_order", "op": "add", "value": -6 }
  ],
  "risks": [
    {
      "id": "local_resistance",
      "probability": 0.25,
      "condition": "resources.public_order < 50"
    }
  ]
}
```

## 13.3 随机性

所有随机必须可复现：

```text
run_seed
step_index
rule_id
random_source
```

---

# 14. 多模态媒体管线

## 14.1 图片类型

| 类型 | 优先级 |
|---|---:|
| Scene 背景图 | P0 |
| 角色肖像 | P0 |
| 事件 CG | P1 |
| 道具/线索图 | P2 |
| UI 主题图 | P2 |

## 14.2 声音类型

| 类型 | 优先级 |
|---|---:|
| NPC 台词 TTS | P1 |
| 旁白朗读 | P1 |
| 环境音 | P2 |
| 音效 | P2 |
| 动态音乐 | P3 |

## 14.3 Asset Registry

```ts
type AssetRecord = {
  id: string;
  kind: "image" | "audio" | "text" | "json";
  path: string;
  hash: string;
  provider?: string;
  promptHash?: string;
  createdAt: number;
  refs: string[];
};
```

资产策略：

- hash 去重。
- 引用计数。
- 未引用资产可清理。
- 生成参数可追溯。
- 导出时只打包被引用资产。

---

# 15. 创作者工作台

## 15.1 信息架构

```text
首页
  ├── 最近项目
  ├── 新建项目
  └── 模板库

项目工作台
  ├── 总览 Dashboard
  ├── 世界设定 World Bible
  ├── 剧情工坊 Story Craft
  ├── 角色 Characters
  ├── 地点 Locations
  ├── 状态变量 State
  ├── 规则 Rules
  ├── 事件 Events
  ├── Agent 配置
  ├── 图像风格 Visual Bible
  ├── 声音 Audio Bible
  ├── 本地试玩 Playtest
  ├── 调试 Debugger
  ├── 素材 Assets
  └── 导出 Export
```

## 15.2 剧情工坊 UI

P0 提供基础表单：

```text
题材承诺
目标读者/玩家情绪
主线问题
开场钩子
核心伏笔
角色弧线
场景目的列表
```

P1 提供可视化：

```text
情绪曲线图
伏笔状态表
角色关系图
Scene 节奏时间线
```

## 15.3 新建项目流程

```text
选择模板
  ↓
填写一句话游戏概念
  ↓
AI 生成 World Bible / Story Bible / Story Craft Bible
  ↓
选择画风
  ↓
选择是否启用语音
  ↓
生成角色与初始场景
  ↓
进入工作台
```

---

# 16. 高级开发者模式

## 16.1 工程文件可直接编辑

高级开发者可以直接打开项目目录：

```text
VS Code
Git
Codex
终端
外部图片/音频工具
```

## 16.2 AGENTS.md

每个项目生成 `AGENTS.md`：

```md
# AGENTS.md

## Project goal
This is an AI multimodal interactive story/game project.

## Commands
- cargo check --workspace
- cargo test --workspace
- pnpm typecheck
- pnpm test

## Rules
- Do not change public schema without updating schema tests.
- Core runtime must not depend on Tauri.
- All AI provider calls must be behind provider traits.
- Never put API keys in frontend code.
- Add snapshot tests for generated JSON validators.
- Story generation must update PlotThread and StoryCraftState when relevant.
```

---

# 17. Steam 产品化与发布路线

## 17.1 核心判断

把产品做成 Steam 游戏是有意思且可行的，但定位必须调整：

```text
不是“帮玩家一键上架 Steam 的工具”
而是“一款玩家在其中创造、游玩、发布 AI 图文游戏的创作型游戏”
```

Steam 内的主要发布渠道应是 Workshop，而不是直接替玩家发布独立 Steam App。

## 17.2 Steam 游戏本体的玩法包装

Steam 游戏名建议统一为：

```text
中文：剧造：游戏工坊
英文：PlotForge: AI Story Game Maker
```

玩家循环：

```text
选择模板
  ↓
创造世界
  ↓
生成角色
  ↓
试玩第一幕
  ↓
调剧情/规则/图片
  ↓
发布到 Workshop
  ↓
游玩/订阅/Remix 其他作品
```

## 17.3 Steam 版的三种模式

| 模式 | 说明 | 优先级 |
|---|---|---:|
| Play Mode | 玩内置作品和 Workshop 作品 | P0 for Steam |
| Creator Mode | 游戏内低门槛创作 | P0 for Steam |
| Developer Mode | 高级文件工程/VS Code/Git | P1 |

## 17.4 Workshop UGC 策略

玩家作品作为 Workshop Item：

```text
作品包：project_manifest + scenes + rules + assets + metadata
预览图：封面或首场景图
标签：题材、语言、是否需要 AI Key、是否含动态生成
订阅：下载到本地作品库
Remix：复制为本地工程
```

MVP 不做付费 Workshop Item。

## 17.5 “玩家上架 Steam”的正确表达

不能承诺：

```text
玩家点击按钮，自动上架 Steam 商店
```

可以提供：

```text
Export Steam Submission Kit
```

内容包括：

```text
独立桌面包
商店文案草稿
AI 使用披露草稿
截图/封面导出
构建说明
Steam Direct checklist
内容安全 checklist
```

最终上架仍由创作者自己：

```text
注册 Steamworks
支付 Steam Direct Fee
提交商店页和 build
完成内容问卷
通过 Valve review
```

## 17.6 Steam AI 内容披露

Steam 内容问卷要求开发者说明生成式 AI 用途。根据 Steamworks 内容问卷，AI 部分关注的是“随游戏发布并被玩家消费”的 AI 生成内容，包括 artwork、sound、narrative、localization 等；Live-generated 内容还需要说明 guardrails 以防生成非法内容。

因此本产品必须支持：

```text
AI Usage Manifest
  - pre_generated_ai_content
  - live_generated_ai_content
  - model providers
  - content categories
  - safety guardrails
  - user reporting path
  - moderation policy
```

Steam 导出包应自动生成 AI 披露草稿，但不替用户作法律保证。

## 17.7 Steam 路线里程碑

| 阶段 | 目标 |
|---|---|
| S0 | 本地桌面工具，不上 Steam |
| S1 | 做 Steam Demo：内置作品 + 创作模式预览 |
| S2 | 支持 Workshop 订阅和加载作品 |
| S3 | 支持 Workshop 上传作品 |
| S4 | 支持 Steam Submission Kit 导出 |
| S5 | 官方云/跨平台分享，视商业情况决定 |

---

# 18. 桌面端技术架构

## 18.1 技术栈

```text
Desktop Shell：Tauri v2
PlotForge Engine：Rust
Frontend：React + TypeScript + Vite + Tailwind
Editor：Monaco Editor
Database：SQLite
Project Format：文件夹工程 + TOML/JSON/Markdown
AI Provider：Rust trait adapter
Runtime Preview：内嵌 WebView
Export：静态 Web 包 + 本地 Runtime 包
```

## 18.2 Rust Workspace

```text
crates/
  plotforge-schema/
  plotforge-core/
  plotforge-runtime/
  plotforge-rule/
  plotforge-storycraft/
  plotforge-agent/
  plotforge-media/
  plotforge-storage/
  plotforge-export/
  plotforge-steam/
  plotforge-cli/
apps/
  creator-desktop/
  player-web/
```

## 18.3 crate 职责

| Crate | 职责 |
|---|---|
| plotforge-schema | 所有数据结构、serde、schema |
| plotforge-runtime | Story / Scene / Beat 运行时 |
| plotforge-rule | 规则引擎、状态变更 |
| plotforge-storycraft | 剧情结构、伏笔、情绪曲线、审稿 |
| plotforge-agent | Agent 编排、Provider 抽象 |
| plotforge-media | 图片、TTS、资产缓存 |
| plotforge-storage | SQLite、文件工程读写 |
| plotforge-export | Web / 桌面导出 |
| plotforge-steam | Workshop / Steam submission kit，P2 |
| plotforge-cli | 命令行调试 |

---

# 19. 项目工程格式

## 19.1 源文件结构

```text
my-game/
  game.toml
  world/
    world.md
    canon.md
    resources.toml
  story/
    story_bible.md
    story_craft.toml
    emotional_arc.json
    plot_threads.toml
    character_arcs.toml
    style_guide.md
  characters/
    chongzhen.character.toml
    wang_chengen.character.toml
  locations/
    qianqing_palace.location.toml
  rules/
    taxation.rule.toml
    corruption.rule.toml
  events/
    rebellion.event.toml
  agents/
    scene_planner.prompt.md
    beat_writer.prompt.md
    plot_doctor.prompt.md
    validator.prompt.md
  references/
    methods/
    user_imports/
    analyses/
  reviews/
    narrative_reviews/
  assets/
    images/
    voices/
    generated/
  saves/
  traces/
  exports/
  AGENTS.md
```

## 19.2 源数据与缓存边界

```text
源数据：TOML / JSON / Markdown
缓存：SQLite
生成资产：assets/generated
运行记录：traces
```

SQLite 不作为唯一数据源。

---

# 20. Provider 与模型接入

## 20.1 Provider 类型

```text
TextModelProvider
ImageProvider
TtsProvider
VisionProvider
ModerationProvider
```

## 20.2 BYO Key

支持两类：

```text
Creator BYO Key：创作端本地生成
Player BYO Key：玩家运行动态作品时自己提供 Key
```

任何导出包不得包含创作者 API Key。

---

# 21. 调试、可观测性与成本控制

## 21.1 Runtime Trace

```ts
type RuntimeTrace = {
  id: string;
  timestamp: number;
  playerInput?: string;
  selectedChoice?: string;

  worldStateBefore: WorldState;
  worldStateDelta?: WorldStateDelta;
  worldStateAfter: WorldState;

  storyStatePatch?: StoryStatePatch;
  storyCraftPatch?: StoryCraftPatch;
  narrativeReview?: NarrativeReview;

  agentCalls: AgentTrace[];
  mediaCalls: MediaTrace[];
  errors: RuntimeError[];
};
```

## 21.2 Job Queue

所有耗时任务都走 Job Queue：

```text
LLM call
Image generation
TTS
Export
Workshop upload
Reference analysis
```

支持：

```text
取消
重试
超时
进度
失败降级
成本统计
```

---

# 22. 导出与发布策略

## 22.1 导出模式

| 模式 | 是否动态 AI | API Key 风险 | MVP |
|---|---:|---:|---:|
| Static Web Export | 否 | 无 | 是 |
| Desktop Runtime Export | 可选 | 低，本地保存 | P1 |
| BYO Key Web Export | 是 | 玩家自担 | P1/P2 |
| Self-host Backend Export | 是 | 创作者后端承担 | P2 |
| Steam Workshop Item | 可选 | 依赖运行策略 | P2 |
| Steam Submission Kit | 可选 | 由创作者处理 | P2 |

## 22.2 MVP 导出

MVP 只做静态 Web 导出：

```text
已生成 Scene
已生成图片
预烘焙文本
本地选择分支
不调用模型
不包含 API Key
```

动态 AI Web 托管不进入 MVP。

---

# 23. 安全、权限、版权与合规

## 23.1 API Key 安全

```text
API Key 只保存在本地安全存储
前端不直接读 Key
导出包不包含 Key
trace 默认脱敏
```

## 23.2 Prompt 注入

玩家输入不能直接进入系统 prompt。必须经过：

```text
Input Sanitizer
Action Interpreter
Policy Check
Context Builder
```

## 23.3 版权与参考库

MVP 禁止默认抓取小说正文或受版权保护内容。

允许：

```text
用户手动导入自己有权使用的文本
公开许可文本
用户自己写的作品
结构化方法论模板
短摘要和结构分析
```

不允许：

```text
自动抓取付费小说正文
内置受版权保护作品全文
生成明显模仿特定在世作者的可识别长篇文本
导出第三方 IP 侵权素材
```

## 23.4 Steam 合规

若走 Steam：

```text
必须填写 Content Survey
必须披露玩家可消费的 AI 生成内容
Live-generated AI 必须有 guardrails
UGC 必须有举报/删除/本地屏蔽机制
商店页截图必须来自实际 gameplay
不承诺尚未实现功能
```

---

# 24. MVP 功能需求

## 24.1 项目管理

### FR-PROJ-001 新建项目

验收：

- 创建项目文件夹。
- 生成 `game.toml`。
- 生成基础目录结构。
- 生成 `story_craft.toml`。
- 可重新打开项目。

## 24.2 世界设定

### FR-WORLD-001 编辑 World Bible

验收：

- Markdown 编辑。
- AI 扩写。
- canon rules。
- forbidden facts。

## 24.3 剧情工坊

### FR-STORY-001 Story Craft Bible

验收：

- 可编辑题材承诺、主线问题、目标情绪、核心伏笔。
- 可生成初始情绪曲线。
- 可生成至少 3 个 PlotThread。

### FR-STORY-002 Narrative Review

验收：

- 每个生成 Scene 后自动生成 narrative review。
- 至少检查弱钩子、无推进、假选择、角色 OOC、伏笔断线。
- 审查失败不阻断，但在 Debugger 显示。

## 24.4 角色

### FR-CHAR-001 创建角色

验收：

- 可手动创建。
- 可 AI 自动生成。
- 可生成角色基础肖像。
- 可保存视觉卡和声音卡。

## 24.5 状态变量

### FR-STATE-001 资源变量

验收：

- 可配置初始值。
- 可配置 min/max。
- 运行时可变化。

## 24.6 规则

### FR-RULE-001 声明式规则

验收：

- 可创建条件。
- 可创建效果。
- 可绑定 action type。
- 不执行任意脚本。

## 24.7 Runtime

### FR-RUNTIME-001 Scene/Beat 推进

验收：

- continue 不生成新图。
- changeScene 生成新 Scene。
- 存档可恢复。

## 24.8 Agent Pipeline

### FR-AGENT-001 Mock Pipeline

验收：

- 不接模型也能跑通。
- CLI 可执行完整 Demo。

### FR-AGENT-002 LLM Pipeline

验收：

- Scene Planner 输出 schema 校验。
- Beat Writer 输出 schema 校验。
- Plot Doctor 输出 narrative review。

## 24.9 Image Pipeline

### FR-IMG-001 Scene 背景图

验收：

- 每个新 Scene 可生成背景图。
- 同一 sceneKey 可复用 reference。
- 失败时显示占位图。

## 24.10 Export

### FR-EXPORT-001 Static Web Export

验收：

- 可导出 zip。
- 不包含 API Key。
- 可本地打开游玩。

---

# 25. 非功能需求

## 25.1 可维护性

```text
核心引擎与 Tauri 解耦
所有 schema 可测试
所有规则可测试
所有 Agent 输出可 mock
```

## 25.2 稳定性

```text
AI JSON 输出必须 repair + validate
Agent 失败必须 fallback
图片失败必须 placeholder
语音失败必须 silent fallback
规则执行失败不能破坏存档
```

## 25.3 可复现性

```text
run_seed
prompt_version
model_version
provider_config_hash
trace
snapshot
```

## 25.4 性能

```text
Scene 内 Beat 推进零网络请求
图片生成异步任务
TTS 懒加载
大资产按需加载
```

---

# 26. 单人 + Codex 开发路线图

## 26.1 总原则

```text
先 CLI，后 UI
先 schema，后模型
先 mock，后真实 Provider
先剧情质量，后多模板
先一个 Demo，后平台化
```

## 26.2 v0.1：Core CLI Prototype

目标：无 UI 跑通核心循环。

任务：

```text
1. Rust workspace
2. plotforge-schema
3. plotforge-runtime
4. plotforge-rule
5. plotforge-storycraft 基础类型
6. mock agent
7. demo project loader
8. CLI play loop
```

## 26.3 v0.2：Story Craft Prototype

目标：让剧情系统成为核心，而不是 prompt 装饰。

任务：

```text
1. StoryCraftBible
2. PlotThread Registry
3. EmotionalArc
4. NarrativeReview
5. PlotDoctor mock
6. ConsistencyChecker mock
```

## 26.4 v0.3：Desktop Shell Prototype

目标：桌面端能打开项目、编辑、试玩。

## 26.5 v0.4：LLM Generation Prototype

目标：接入真实 LLM 生成 Scene Plan / Beats / Review。

## 26.6 v0.5：Image Prototype

目标：Scene 能生成图并缓存。

## 26.7 v0.6：Simulation Demo

目标：《王朝余烬》跑通。

## 26.8 v0.7：Static Export

目标：导出静态 Web 包。

## 26.9 v0.8：Steam Demo Exploration

目标：评估是否包装为 Steam 创作型游戏。

---

# 27. 首个内置 Demo：王朝余烬

## 27.1 定位

```text
类型：历史危机模拟 + 剧情驱动
玩家身份：末代皇帝
核心循环：看国情 → 问大臣 → 下诏 → 推演后果 → 新危机
```

## 27.2 初始资源

```text
国库 treasury
民心 public_order
军心 army_morale
朝堂稳定 court_stability
地方控制 local_control
外敌压力 enemy_pressure
```

## 27.3 初始角色

```text
首辅
兵部尚书
司礼监太监
边将
言官
地方总督
```

## 27.4 Story Craft 目标

```text
主线问题：玩家能否在内忧外患中延续王朝？
情绪承诺：权力压力、孤独决策、短期收益与长期代价
核心伏笔：边军真实军饷缺口、朝中内鬼、地方税乱、迁都争议
场景节奏：每 2-3 个 Scene 出现一次明确危机升级
```

## 27.5 示例玩家行动

```text
朕决定加征辽饷，同时命锦衣卫严查贪墨官员。
```

## 27.6 示例状态变化

```json
{
  "treasury": 12,
  "public_order": -8,
  "court_stability": -5,
  "army_morale": 4,
  "triggered_events": ["local_tax_resistance", "officials_submit_memorials"]
}
```

---

# 28. 风险与应对

## 28.1 范围过大

应对：第一版只做《王朝余烬》一个模板。

## 28.2 剧情空洞

应对：Story Craft Pipeline P0，Plot Doctor P0。

## 28.3 AI 输出不稳定

应对：schema validate、fallback、trace。

## 28.4 Steam 路线误判

应对：先 Workshop，后 Steam Submission Kit；不承诺一键上架商店。

## 28.5 版权风险

应对：不默认抓取版权正文；参考库只存结构和用户授权资料。

## 28.6 动态 AI 内容合规

应对：AI Usage Manifest、内容过滤、举报机制、可关闭 live generation。

---

# 29. 待决问题

## 29.1 品牌名后续检查

当前推荐品牌名已确定：

```text
中文：剧造
英文：PlotForge
完整：剧造 PlotForge
```

正式公开前仍需完成：

```text
商标检索
Steam / itch / GitHub / 域名可用性检查
中英文 Logo 可读性测试
海外同名产品冲突检查
```

## 29.2 首发形态

建议：

```text
先桌面工具 alpha
再 Steam Demo
最后 Workshop UGC
```

## 29.3 第一批模型 Provider

建议：

```text
OpenAI-compatible text
OpenAI-compatible image / Runware / local placeholder
TTS P1 后接
```

## 29.4 剧情参考库边界

需要明确：

```text
内置方法论可以
内置版权正文不可以
用户导入需要免责声明和本地处理
```

---

# 30. 附录：建议的首批 Codex 任务

## Task 001：创建 Rust workspace

```text
Create a Rust workspace with crates:
- plotforge-schema
- plotforge-runtime
- plotforge-rule
- plotforge-storycraft
- plotforge-agent
- plotforge-storage
- plotforge-cli

Add cargo check --workspace and basic CI config.
```

## Task 002：实现 schema crate

```text
Implement GameProject, WorldState, StoryState, StoryCraftState, Scene, Beat, Choice, Character schemas.
All types must derive Serialize, Deserialize, Clone, Debug.
Add JSON roundtrip tests.
```

## Task 003：实现 Story Craft MVP

```text
Implement PlotThread, EmotionalArc, NarrativeReview, NarrativeIssue.
Add a mock PlotDoctor that checks:
- missing dramaticPurpose
- no hook
- no plotThread update
- fake choice
```

## Task 004：实现 Rule Engine MVP

```text
Implement declarative rule engine:
- condition matching
- resource add/set
- flag set
- deterministic random risk trigger
Add tests.
```

## Task 005：实现 mock runtime

```text
Load demo project.
Start session.
Show first scene.
Apply one player action.
Generate next mock scene.
Write runtime trace.
```

## Task 006：实现 CLI

```text
Commands:
- plotforge new demo
- plotforge check
- plotforge play
- plotforge trace inspect
```

---

# 31. 参考资料

- InfiPlot GitHub: https://github.com/zonghaoyuan/infiplot
- oh-story-claudecode GitHub: https://github.com/worldwonderer/oh-story-claudecode
- Steamworks Content Survey: https://partner.steamgames.com/doc/gettingstarted/contentsurvey
- Steam Direct Fee: https://partner.steamgames.com/doc/gettingstarted/appfee
- Steam Review Process: https://partner.steamgames.com/doc/store/review_process
- Steam Workshop: https://partner.steamgames.com/doc/features/workshop
- Tauri Documentation: https://tauri.app/
- Cargo Workspaces: https://doc.rust-lang.org/cargo/reference/workspaces.html

