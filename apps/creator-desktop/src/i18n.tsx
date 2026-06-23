import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export const supportedLocales = ["en", "zh"] as const;
export type StudioLocale = (typeof supportedLocales)[number];

const storageKey = "plotforge:creator-desktop:locale";

const zhText: Record<string, string> = {
  "PlotForge Studio": "PlotForge Studio",
  "Creator Desktop": "创作者桌面端",
  "Open Project": "打开项目",
  "Agent-native workflows": "Agent 原生工作流",
  "Workflow surfaces": "工作流界面",
  "Evidence panel": "证据面板",
  "Command dock": "命令栏",
  "Command Dock": "命令栏",
  "Command Center": "命令中心",
  "Director Mode": "导演模式",
  "Agent Mesh": "Agent 网格",
  "Artifact Review": "产物审查",
  "Playable Proof": "可玩证明",
  "Export Package": "导出包",
  Launchpad: "启动台",
  "World Bible": "世界设定",
  "Story Craft": "故事工艺",
  Characters: "角色",
  State: "状态",
  Rules: "规则",
  Assets: "资产",
  Playtest: "试玩",
  Debugger: "调试器",
  Export: "导出",
  ready: "就绪",
  next: "下一步",
  later: "稍后",
  Save: "保存",
  Refresh: "刷新",
  "Run proof": "运行证明",
  "Run playable proof": "运行可玩证明",
  "Run turn": "运行回合",
  "Apply as proof run": "应用为证明运行",
  "Project path": "项目路径",
  "Refresh project files": "刷新项目文件",
  "New Project": "新建项目",
  "Folder-backed project scaffold": "基于文件夹的项目脚手架",
  "Create project": "创建项目",
  Template: "模板",
  "Historical Crisis": "历史危机",
  "Dynasty Embers": "王朝余烬",
  "Visual style": "视觉风格",
  Concept: "概念",
  "Initial scene": "初始场景",
  "Initial scene request": "初始场景请求",
  "Voice enabled": "启用语音",
  "Overwrite existing path": "覆盖已有路径",
  "Creation Report": "创建报告",
  Project: "项目",
  Files: "文件",
  "No project created in this session.": "本次会话尚未创建项目。",
  "No project loaded": "未加载项目",
  "Project loaded": "项目已加载",
  "No project": "无项目",
  "Unsaved source": "源文件未保存",
  "Workspace synced": "工作区已同步",
  "Project Launchpad": "项目启动台",
  "Playable Proof Status": "可玩证明状态",
  "Playable proof captured": "已捕获可玩证明",
  "Ready to run proof": "可运行证明",
  "No turn has been run in this session.": "本会话尚未运行回合。",
  "Export Readiness": "导出就绪度",
  "Static package available": "静态包可用",
  "No static package profile": "没有静态包配置",
  "Export profile not loaded": "导出配置未加载",
  "Live Game Canvas": "实时游戏画布",
  "Runtime Trace": "运行时追踪",
  "Scene preview": "场景预览",
  "Open a project to preview the playable scene and proof loop.":
    "打开项目以预览可玩场景和证明循环。",
  "Director Command Input": "导演命令输入",
  "Director intent": "导演意图",
  "Recent Runs": "最近运行",
  "State Delta": "状态变化",
  "Backend Boundary": "后端边界",
  "Real Studio command surface": "真实 Studio 命令面",
  "Backend Bridge": "后端桥接",
  "Real Studio commands": "真实 Studio 命令",
  "Project truth": "项目事实源",
  "folder source files": "文件夹源文件",
  "Command source": "命令来源",
  "Network ACP": "网络 ACP",
  "Provider calls": "Provider 调用",
  "not implemented": "未实现",
  disabled: "已禁用",
  "Removed Fake Surfaces": "已移除模拟界面",
  "Command Boundary Map": "命令边界图",
  "Capability Matrix": "能力矩阵",
  "Studio-backed capabilities": "Studio 支撑的能力",
  Capability: "能力",
  "Real source": "真实来源",
  Evidence: "证据",
  Status: "状态",
  "Project Source": "项目来源",
  "Runtime Evidence": "运行时证据",
  "Export Evidence": "导出证据",
  "Source Files": "源文件",
  "Asset Records": "资产记录",
  "Source Artifacts": "源产物",
  "Artifact Text Editor": "产物文本编辑器",
  "No source file selected": "未选择源文件",
  modified: "已修改",
  "Boundary Checks": "边界检查",
  "Error visible": "错误可见",
  "Trace visible": "追踪可见",
  "External Agents": "外部 Agent",
  "External agents": "外部 Agent",
  "Folder files win over cache": "文件夹文件优先于缓存",
  "Generated only by play_once": "仅由 play_once 生成",
  "Source file read/write": "源文件读写",
  "Runtime proof": "运行时证明",
  "Static export zip": "静态导出 zip",
  "ACP / external agent bridge": "ACP / 外部 Agent 桥接",
  "No schema-backed Studio command": "没有 schema 支撑的 Studio 命令",
  "Not exposed by Tauri or HTTP dev bridge": "未通过 Tauri 或 HTTP dev bridge 暴露",
  wired: "已接线",
  "Project open/check": "项目打开/检查",
  "Structured editing": "结构化编辑",
  "Generated contracts": "生成的 contracts",
  "Rust core boundary": "Rust 核心边界",
  "UI adapter only": "仅 UI 适配器",
  "Tauri bridge": "Tauri 桥接",
  "commands wired": "命令已接线",
  "Validation Evidence": "验证证据",
  "Real command outputs": "真实命令输出",
  visible: "可见",
  "Project source": "项目来源",
  "folder files": "文件夹文件",
  Runtime: "运行时",
  "Loaded path": "已加载路径",
  "Source files": "源文件",
  "Asset records": "资产记录",
  "not run": "未运行",
  "not captured": "未捕获",
  "not exported": "未导出",
  none: "无",
  "Reference Screens": "参考界面",
  "Open project": "打开项目",
  "Language": "语言",
  English: "English",
  Chinese: "中文",
  "Activity Stream": "活动流",
  Live: "实时",
  "real data": "真实数据",
  "Folder project loaded": "文件夹项目已加载",
  "Open a project before running a playable turn.":
    "请先打开项目，再运行可玩回合。",
  "Latest playtest committed": "最新试玩已提交",
  "No playtest run yet": "尚未运行试玩",
  "Run turn calls the Studio runtime command and writes a trace.":
    "运行回合会调用 Studio 运行时命令并写入追踪。",
  "Creative Goal": "创作目标",
  "Frame the next playable change.": "设定下一次可玩变更。",
  "Refine goal": "细化目标",
  "Playable Scene Preview": "可玩场景预览",
  "Playable Scene": "可玩场景",
  Play: "播放",
  Reload: "重新加载",
  "Open a project to preview the player-facing scene.":
    "打开项目以预览玩家侧场景。",
  Scene: "场景",
  Tension: "张力",
  Branching: "分支",
  "Direction Bar": "导演指令栏",
  "Run a runtime turn": "运行一次运行时回合",
  "Trace evidence": "追踪证据",
  "Playtest input": "试玩输入",
  "Save ID": "保存 ID",
  "Playtest save id": "试玩保存 ID",
  "Restore ID": "恢复 ID",
  "Playtest restore id": "试玩恢复 ID",
  "Restore latest save": "恢复最新存档",
  "Restore latest": "恢复最新",
  "raise stakes": "提高风险",
  "add clue": "添加线索",
  "make choice consequence visible": "让选择后果可见",
  "test alternate ending": "测试另一种结局",
  "Decision Queue": "决策队列",
  "No decision queue is available. Run a turn to create runtime evidence; agent approval queues are not implemented.":
    "当前没有可用的决策队列。运行一个回合以生成运行时证据；Agent 审批队列尚未实现。",
  "Trace path": "追踪路径",
  "Open trace": "打开追踪",
  "Review evidence": "审查证据",
  "Playtest result": "试玩结果",
  "Needs review": "需要审查",
  Informational: "信息",
  "Scene preview asset unavailable": "场景预览资产不可用",
  "No background asset is declared for this scene.":
    "此场景未声明背景资产。",
  "Asset Maintenance": "资产维护",
  "Visual Bible": "视觉设定",
  "Audio Bible": "音频设定",
  "Save Visual Bible": "保存视觉设定",
  "Save Audio Bible": "保存音频设定",
  "No asset records or scene background paths found.":
    "未找到资产记录或场景背景路径。",
  "No Visual Bible style cards in project data.":
    "项目数据中没有视觉设定风格卡。",
  "No Audio Bible voice cards in project data.":
    "项目数据中没有音频设定语音卡。",
  References: "引用",
  "Visual cards": "视觉卡",
  "Audio cards": "音频卡",
  Prompt: "提示词",
  Palette: "调色板",
  Tags: "标签",
  "Reference assets": "引用资产",
  Voice: "语音",
  Delivery: "演绎方式",
  "Sample text": "示例文本",
  style: "风格",
  voice: "语音",
  "Artifact Review Workspace": "产物审查工作区",
  "Live Build Room": "实时构建室",
  "No build run interface": "没有构建运行界面",
  "Real Studio commands expose source files, assets, runtime proof, and export reports. They do not expose an agent build queue yet.":
    "真实 Studio 命令会暴露源文件、资产、运行时证明和导出报告，但尚未暴露 Agent 构建队列。",
  "Run a playable proof to generate trace evidence.":
    "运行可玩证明以生成追踪证据。",
  "Run a static export to inspect package evidence.":
    "运行静态导出以检查包证据。",
  "This view now reviews artifacts returned by real Studio commands. Agent-generated patch bundles are hidden until a real persisted artifact interface exists.":
    "此视图现在审查真实 Studio 命令返回的产物。在真实持久化产物接口存在前，Agent 生成的补丁包会保持隐藏。",
  "project source": "项目来源",
  Editable: "可编辑",
  Trace: "追踪",
  "No approval action is available because there is no real approval queue or persisted proposal bundle contract.":
    "当前没有可用审批操作，因为还没有真实审批队列或持久化提案包 contract。",
  "Current Source Artifacts": "当前源产物",
  editable: "可编辑",
  "read only": "只读",
  Bytes: "字节",
  Source: "来源",
  "Loaded through StudioDataSource list_source_files":
    "通过 StudioDataSource list_source_files 加载",
  "Runtime Impact": "运行时影响",
  "No runtime proof has been run for this session.":
    "本会话尚未运行运行时证明。",
  available: "可用",
  Output: "输出",
  Archive: "归档",
  "not archived": "未归档",
  "Asset registry": "资产注册表",
  "Runtime trace": "运行时追踪",
  "Static package": "静态包",
  "No trace captured yet.": "尚未捕获追踪。",
  "No export report captured yet.": "尚未捕获导出报告。",
  "Available Actions": "可用操作",
  "Running proof": "正在运行证明",
  "View trace": "查看追踪",
  // agents.* namespace (AgentMeshView)
  "Studio Backend Bridge": "Studio 后端桥接",
  explicit: "明确",
  "No mock external workers or mock connected state.":
    "没有模拟外部 worker 或模拟连接状态。",
  "No local approval queue unless a real command exists.":
    "除非真实命令存在，否则不显示本地审批队列。",
  "No generated artifact bundle without persisted evidence.":
    "没有持久化证据时，不显示生成的产物包。",
  "The UI is now backed by Studio command results. Agent and ACP concepts stay visible only as unavailable boundaries until schema-backed ports are added.":
    "此 UI 现在由 Studio 命令结果支撑。在加入 schema 支撑的端口之前，Agent 和 ACP 概念只作为不可用边界展示。",
  "Only static web has an executable Studio command":
    "只有静态 Web 有可执行的 Studio 命令",
  "Bridge Evidence": "桥接证据",
  "Current backend facts": "当前后端事实",
  "Playable proof": "可玩证明",
  "Trace id": "追踪 ID",
  "Safety Boundary": "安全边界",
  "No provider credentials are read by this UI surface.":
    "此 UI 界面不会读取 provider 凭据。",
  "No external agent connection is started from the browser.":
    "浏览器不会启动外部 Agent 连接。",
  "Unsupported capabilities are disabled instead of simulated.":
    "不支持的能力会被禁用，而不是被模拟。",
  "Browser mode uses the HTTP dev bridge backed by plotforge-studio.":
    "浏览器模式使用由 plotforge-studio 支撑的 HTTP dev bridge。",
  "Tauri mode uses the same command names through IPC.":
    "Tauri 模式通过 IPC 使用相同的命令名。",
  "ACP workers, approval queues, provider calls, and publishing automation are not implemented.":
    "ACP worker、审批队列、provider 调用和发布自动化尚未实现。",
  Actions: "操作",
  "Review trace": "审查追踪",
  "Describe the game change you want": "描述你想要的游戏变更",
  "PlotForge sends this through the real runtime playtest command today; agent proposal workflows are not implemented.":
    "PlotForge 当前会通过真实运行时试玩命令发送该内容；Agent 提案工作流尚未实现。",
  "Director Brief": "导演简报",
  "Load a folder project to direct the next playable change.":
    "加载文件夹项目以导演下一次可玩变更。",
  "Source artifact": "源产物",
  "Entry scene": "入口场景",
  "Backend Commands": "后端命令",
  "open_project, check_project, source file read/write":
    "open_project、check_project、源文件读写",
  "play_once_project writes redaction-safe trace evidence":
    "play_once_project 会写入脱敏安全的追踪证据",
  "export_static_project_zip writes a whitelisted package":
    "export_static_project_zip 会写入白名单导出包",
  captured: "已捕获",
  "profile loaded": "配置已加载",
  "not loaded": "未加载",
  "Unavailable Agent Interfaces": "不可用的 Agent 接口",
  "ACP workers are not connected by any Studio command.":
    "ACP worker 尚未通过任何 Studio 命令连接。",
  "Approval queues are hidden until persisted proposal contracts exist.":
    "在持久化提案 contract 存在前，审批队列会保持隐藏。",
  "Provider-backed generation remains explicit local mock/runtime logic.":
    "Provider 支撑的生成仍然是明确的本地 mock/运行时逻辑。",
  "Evidence Snapshot": "证据快照",
  Backend: "后端",
  Proof: "证明",
  Artifacts: "产物",
  current: "当前",
  "waiting for first proof run": "等待首次证明运行",
  Scenes: "场景",
  "Open Threads": "开放线索",
  "review notes": "审查备注",
  promises: "承诺",
  "Media References": "媒体引用",
  "Review Issues": "审查问题",
  Diagnostics: "诊断",
  Errors: "错误",
  "No trace errors": "没有追踪错误",
  "No world delta": "没有世界状态变化",
  "World Delta": "世界状态变化",
  true: "是",
  false: "否",
  Command: "命令",
  Game: "游戏",
  Agents: "Agent",
  "Agent Mesh Core": "Agent 网格核心",
  "ACP Bridge Setup": "ACP 桥接设置",
  "Trace Debug": "追踪调试",
  "Director intent, project health, artifact summary":
    "导演意图、项目健康度、产物摘要",
  "ACP capability map, approvals, and local boundaries":
    "ACP 能力图、审批和本地边界",
  "Canon, forbidden facts, setting notes": "正典、禁用事实、设定笔记",
  "Promises, hooks, reversals, emotional arc":
    "承诺、钩子、反转、情绪弧线",
  "Roles, arcs, visual and voice cards": "角色、弧线、视觉和语音卡",
  "Resources, flags, triggered events": "资源、标记、触发事件",
  "Declarative conditions and effects": "声明式条件和效果",
  "Generated and imported files": "生成和导入的文件",
  "Local runtime preview": "本地运行时预览",
  "Trace, diagnostics, review notes": "追踪、诊断、审查备注",
  "Static web and package profiles": "静态 Web 和包配置",
  "Director intent, project launch, active run summary":
    "导演意图、项目启动、活动运行摘要",
  "Playable scene preview, creative direction, runtime loop":
    "可玩场景预览、创作指令、运行时循环",
  "Local capability map, ACP setup, approval boundaries":
    "本地能力图、ACP 设置、审批边界",
  "Agent proposals, changed assets, validation evidence":
    "Agent 提案、变更资产、验证证据",
  "Trace-visible playtest evidence and reproducibility":
    "追踪可见的试玩证据和可复现性",
  "Local package readiness, manifests, disclosure drafts":
    "本地包就绪度、manifest、披露草稿",
  "Director Intent": "导演意图",
  "Proof And Trace Workspace": "证明与追踪工作区",
  "Proof Evidence Panel": "证明证据面板",
  "Player files": "播放器文件",
  ExportManifest: "ExportManifest",
  "Reachable assets": "可达资产",
  "Story and rules data": "故事和规则数据",
  "AI usage disclosure": "AI 使用披露",
  "Content warning draft": "内容警告草稿",
  "Archive manifest": "归档 manifest",
  "Local smoke evidence": "本地冒烟证据",
  "No provider configuration": "没有 provider 配置",
  "No private traces": "没有私有追踪",
  "Run a playtest turn to create proof": "运行试玩回合以创建证明",
  "Trace output appears after a playtest turn.":
    "试玩回合完成后会显示追踪输出。",
  "Playable result, state deltas, run evidence, artifact diff, and package readiness are shown from the current local run.":
    "当前本地运行会显示可玩结果、状态变化、运行证据、产物差异和包就绪度。",
  Fallback: "回退",
  Committed: "已提交",
  Waiting: "等待中",
  "no run": "未运行",
  "no trace": "无追踪",
  "No playable result": "没有可玩结果",
  "Run a proof turn to inspect the player-facing result.":
    "运行一次证明回合以检查玩家侧结果。",
  "Trace ID": "追踪 ID",
  "Run seed": "运行种子",
  "Run Seed": "运行种子",
  "State deltas": "状态变化",
  "Package readiness": "包就绪度",
  "local ready": "本地就绪",
  "review required": "需要审查",
  "Run Evidence": "运行证据",
  "Selected choice": "已选选择",
  Intent: "意图",
  "Prompt version": "提示词版本",
  "Prompt Version": "提示词版本",
  Snapshot: "快照",
  "Redaction-safe causality": "脱敏安全因果链",
  "Run a playtest turn to inspect trace evidence.":
    "运行试玩回合以检查追踪证据。",
  "Agent Evidence": "Agent 证据",
  "No trace selected": "未选择追踪",
  "Provider Config Hash": "Provider 配置哈希",
  "Package Evidence Summary": "包证据摘要",
  "Files written": "已写入文件",
  "Archived files": "已归档文件",
  "Allowed files": "允许文件",
  "No export report captured.": "尚未捕获导出报告。",
  "AI Usage Disclosure": "AI 使用披露",
  "Content Warning Draft": "内容警告草稿",
  "AI usage disclosure appears after policy load.":
    "策略加载后会显示 AI 使用披露。",
  "Content warning draft appears after policy load.":
    "策略加载后会显示内容警告草稿。",
  "Local draft only": "仅本地草稿",
  "Local Static Web Package": "本地静态 Web 包",
  "No profile selected": "未选择配置",
  Ready: "就绪",
  Review: "审查",
  matched: "已匹配",
  mismatch: "不匹配",
  pending: "待处理",
  "Trace Evidence": "追踪证据",
  "Action Intent": "动作意图",
  "Rule Result": "规则结果",
  "Planner Result": "规划结果",
  Reproducibility: "可复现性",
  Choice: "选择",
  Action: "动作",
  "Matched terms": "匹配词",
  Reason: "原因",
  "Delta empty": "变化为空",
  "Error code": "错误代码",
  Error: "错误",
  Requested: "请求",
  "Model version": "模型版本",
  "Provider config hash": "Provider 配置哈希",
  "Snapshot id": "快照 ID",
  "Causality Graph": "因果图",
  "Rules.patch": "规则补丁",
  "Runtime.playtest": "运行时试玩",
  "not committed": "未提交",
  "flows to": "流向",
  "Tool Metadata": "工具元数据",
  "Prompt hash": "提示词哈希",
  "Tool calls": "工具调用",
  "Export files": "导出文件",
  Hook: "钩子",
  Pacing: "节奏",
  Character: "角色",
  Payoff: "回收",
  "AI slop": "AI 味风险",
  "Narrative Review": "叙事审查",
  "No media references": "没有媒体引用",
  "No runtime errors": "没有运行时错误",
  "No project loaded - Local package readiness, manifests, disclosure drafts":
    "未加载项目 - 本地包就绪度、manifest、披露草稿",
  "No raw responses": "没有原始响应",
  "No secret markers": "没有密钥标记",
  "All referenced assets copied": "所有引用资产已复制",
  "No absolute machine paths": "没有机器绝对路径",
  "HTTP smoke test passed": "HTTP 冒烟测试已通过",
  Scripts: "脚本",
  "player bundle": "播放器包",
  "Select resource": "选择资源",
  // export.* namespace
  "Export zip": "导出 zip",
  "Evidence & Boundaries": "证据与边界",
  "Local export package only": "仅本地导出包",
  // trace.* namespace
  "Run Result Summary": "运行结果摘要",
  "Technical Details": "技术详情",
  "These checks remain pending until a separate smoke test is run after export.":
    "这些检查在导出后运行独立冒烟测试前将保持待处理状态。",
  "Package Information": "包信息",
  "Package hash": "包哈希",
  "Validation Summary": "验证摘要",
  "Checks passed": "已通过检查",
  "All local boundary checks passed": "所有本地边界检查已通过",
  "Review profile boundary checks": "请审查配置边界检查",
  "Package Contents": "包内容",
  "Playable Preview": "可玩预览",
  "Trace: pending": "追踪：待处理",
  "Dependency Map": "依赖图",
  resolved: "已解析",
  "Size Breakdown": "大小细分",
  Images: "图片",
  Other: "其他",
  Executable: "可执行",
  "Selected Profile": "已选配置",
  "Runtime network": "运行时网络",
  required: "需要",
  "not required": "不需要",
  "Provider config": "Provider 配置",
  included: "已包含",
  excluded: "已排除",
  "Private traces": "私有追踪",
  "Submission ready": "提交就绪",
  claimed: "已声明",
  "not claimed": "未声明",
  Capabilities: "能力",
  "Output directory": "输出目录",
  "Zip archive": "Zip 归档",
  "Export disclosure evidence": "导出披露证据",
  "Live generated content enabled": "启用实时生成内容",
  "Human review required": "需要人工审核",
  "Moderation queue enabled": "启用内容审核队列",
  "Content kinds": "内容类型",
  "Reporting path": "举报路径",
  "Moderation policy": "审核策略",
  "Safety guardrails": "安全防护栏",
  files: "个文件",
  // world.* namespace
  "Save World Bible": "保存世界设定",
  "World bible markdown": "世界设定 Markdown",
  "Canon markdown": "正典 Markdown",
  "Forbidden facts": "禁用事实",
  "AI expansion goal": "AI 扩展目标",
  "World generation goal": "世界生成目标",
  "Generate World Expansion": "生成世界扩展",
  "World edit document not loaded.": "世界编辑文档未加载。",
  // story.* namespace
  "Save Story Craft": "保存故事工艺",
  "Story bible markdown": "故事设定 Markdown",
  "Style guide markdown": "风格指南 Markdown",
  "Story Bible": "故事设定",
  "Style Guide": "风格指南",
  "Genre promise": "类型承诺",
  "Central question": "核心问题",
  "Target emotions": "目标情绪",
  "Core foreshadowing": "核心铺垫",
  "AI story concept": "AI 故事概念",
  "Story generation concept": "故事生成概念",
  "Generate StoryCraft": "生成故事工艺",
  "Story Craft edit document not loaded.": "故事工艺编辑文档未加载。",
  "Story Craft source editing is available in the source editor. Use the AI generation concept above to expand or regenerate the full story craft document.":
    "故事工艺源文件编辑可在源代码编辑器中进行。请使用上方的 AI 生成概念来扩展或重新生成完整的故事工艺文档。",
  // characters.* namespace
  "Add Character": "添加角色",
  "Save Characters": "保存角色",
  "Create Character": "创建角色",
  "Generate Character": "生成角色",
  "AI Generate": "AI 生成",
  Manual: "手动",
  "Character id": "角色 ID",
  "Character generation concept": "角色生成概念",
  "Character generation role hint": "角色生成角色提示",
  "AI character concept": "AI 角色概念",
  "Role hint": "角色提示",
  "Visual card": "视觉卡",
  "Voice card": "语音卡",
  "Portrait request": "肖像请求",
  "New character id": "新角色 ID",
  "New character name": "新角色名称",
  "New character role": "新角色职责",
  "New character traits": "新角色特征",
  "New visual card": "新视觉卡",
  "New voice card": "新语音卡",
  "Character edit document not loaded.": "角色编辑文档未加载。",
  // state.* namespace
  "Save State": "保存状态",
  "Add Resource": "添加资源",
  "Initial Story State": "初始故事状态",
  "The starting scene and turn for a new game session.": "新游戏会话的初始场景和回合。",
  "Create Resource": "创建资源",
  "Current scene": "当前场景",
  "Default initial value": "默认初始值",
  "World initial value": "世界初始值",
  "Resource key": "资源键",
  "New resource key": "新资源键",
  "New resource label": "新资源标签",
  "New resource default initial value": "新资源默认初始值",
  "New resource min": "新资源最小值",
  "New resource max": "新资源最大值",
  "State edit document not loaded.": "状态编辑文档未加载。",
  // rules.* namespace
  "Add Rule": "添加规则",
  "Save Rules": "保存规则",
  "Create Rule": "创建规则",
  "Rule id": "规则 ID",
  "Action type": "动作类型",
  "New rule id": "新规则 ID",
  "New rule action type": "新规则动作类型",
  "New rule resource": "新规则资源",
  "New rule amount": "新规则数量",
  "Note: only add_resource effect is supported when creating rules manually. Edit the rules TOML file directly for complex conditions and effects.":
    "注意：手动创建规则时仅支持 add_resource 效果。如需复杂条件和效果，请直接编辑规则 TOML 文件。",
  "Rule edit document not loaded.": "规则编辑文档未加载。",
  Conditions: "条件",
  Effects: "效果",
  Amount: "数量",
  Resource: "资源",
  "(none)": "（无）",
  // director.* namespace
  "Advanced snapshot controls": "高级快照控制",
};

const zhPatterns: Array<[RegExp, (...matches: string[]) => string]> = [
  [/^(\d+) files$/, (count) => `${count} 个文件`],
  [/^(\d+) choices$/, (count) => `${count} 个选择`],
  [/^(\d+) resources$/, (count) => `${count} 个资源`],
  [/^(\d+) rules$/, (count) => `${count} 条规则`],
  [/^(\d+) source files$/, (count) => `${count} 个源文件`],
  [/^(\d+) listed files from storage adapter\.$/, (count) => `${count} 个来自 storage adapter 的文件。`],
  [/^(\d+) editable surfaces loaded from the project\.$/, (count) => `项目中已加载 ${count} 个可编辑界面。`],
  [/^(\d+) asset records$/, (count) => `${count} 条资产记录`],
  [/^(\d+) style cards$/, (count) => `${count} 张风格卡`],
  [/^(\d+) voice cards$/, (count) => `${count} 张语音卡`],
  [/^(\d+) records$/, (count) => `${count} 条记录`],
  [/^(\d+) asset records from plotforge-media\/storage\.$/, (count) => `${count} 条来自 plotforge-media/storage 的资产记录。`],
  [/^(\d+) scene background fallbacks$/, (count) => `${count} 个场景背景回退`],
  [/^(\d+) wired$/, (count) => `${count} 项已接线`],
  [/^(\d+) wired capabilities$/, (count) => `${count} 项能力已接线`],
  [/^(\d+) profiles$/, (count) => `${count} 个配置`],
  [/^(\d+) errors$/, (count) => `${count} 个错误`],
  [/^(\d+) deltas$/, (count) => `${count} 条变化`],
  [/^(\d+) state deltas produced\.$/, (count) => `产生了 ${count} 条状态变化。`],
  [/^(\d+) files archived\.$/, (count) => `已归档 ${count} 个文件。`],
  [/^(\d+) files found after export\.$/, (count) => `导出后找到 ${count} 个文件。`],
  [/^(\d+) scenes, (\d+) rules, (\d+) characters$/, (scenes, rules, characters) => `${scenes} 个场景，${rules} 条规则，${characters} 个角色`],
  [/^(\d+) visible state deltas from (.+)$/, (count, scene) => `${scene} 产生 ${count} 条可见状态变化`],
  [/^(.+) produced (\d+) visible state deltas\.$/, (scene, count) => `${scene} 产生了 ${count} 条可见状态变化。`],
  [/^(.+) \/ (\d+) deltas$/, (trace, count) => `${trace} / ${count} 条变化`],
  [/^(.+) \/ waiting for first proof run$/, (project) => `${project} / 等待首次证明运行`],
  [/^(\d+) review notes$/, (count) => `${count} 条审查备注`],
  [/^(\d+) promises$/, (count) => `${count} 条承诺`],
  [/^(.+): io error at (.+): No such file or directory \(os error 2\)$/, (project, path) => `${project}: 在 ${path} 发生 IO 错误：没有这个文件或目录（os error 2）`],
];

const textSource = new WeakMap<Text, string>();
const attributeSource = new WeakMap<Element, Map<string, string>>();

interface StudioI18nContextValue {
  locale: StudioLocale;
  setLocale(locale: StudioLocale): void;
  t(value: string): string;
}

const StudioI18nContext = createContext<StudioI18nContextValue | null>(null);

export function StudioI18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<StudioLocale>(() =>
    readInitialLocale(globalThis.window),
  );
  const value = useMemo(
    () => ({
      locale,
      setLocale(nextLocale: StudioLocale) {
        setLocaleState(nextLocale);
        storageFor(globalThis.window)?.setItem(storageKey, nextLocale);
      },
      t(value: string) {
        return translate(value, locale);
      },
    }),
    [locale],
  );

  useEffect(() => {
    const root = globalThis.document?.body;
    if (!root) {
      return;
    }
    root.dataset.locale = locale;
    localizeTree(root, locale);
    const observer = new MutationObserver(() => localizeTree(root, locale));
    observer.observe(root, {
      attributes: true,
      attributeFilter: ["aria-label", "placeholder", "title"],
      characterData: true,
      childList: true,
      subtree: true,
    });
    return () => observer.disconnect();
  }, [locale]);

  return (
    <StudioI18nContext.Provider value={value}>
      {children}
    </StudioI18nContext.Provider>
  );
}

export function useStudioI18n() {
  const context = useContext(StudioI18nContext);
  if (!context) {
    throw new Error("StudioI18nProvider is required.");
  }
  return context;
}

export function LanguageToggle() {
  const { locale, setLocale, t } = useStudioI18n();
  return (
    <label className="inline-flex h-10 items-center gap-2 rounded-md border border-graphite-700/20 bg-canvas-50 px-3 text-sm font-semibold text-ink">
      <span>{t("Language")}</span>
      <select
        aria-label={t("Language")}
        value={locale}
        onChange={(event) => setLocale(event.target.value as StudioLocale)}
        className="bg-transparent text-sm font-semibold outline-none"
      >
        <option value="en">{t("English")}</option>
        <option value="zh">{t("Chinese")}</option>
      </select>
    </label>
  );
}

export function translate(value: string, locale: StudioLocale): string {
  if (locale === "en") {
    return value;
  }
  const compact = value.replace(/\s+/g, " ").trim();
  if (!compact) {
    return value;
  }
  const exact = zhText[compact];
  if (exact) {
    return value.replace(compact, exact);
  }
  for (const [pattern, render] of zhPatterns) {
    const match = compact.match(pattern);
    if (match) {
      return render(...match.slice(1));
    }
  }
  return value;
}

function readInitialLocale(view: Window | undefined): StudioLocale {
  const stored = storageFor(view)?.getItem(storageKey);
  if (isStudioLocale(stored)) {
    return stored;
  }
  if (typeof process !== "undefined" && process.env.NODE_ENV === "test") {
    return "en";
  }
  return "zh";
}

function storageFor(view: Window | undefined): Storage | null {
  const storage = view?.localStorage;
  if (
    storage &&
    typeof storage.getItem === "function" &&
    typeof storage.setItem === "function"
  ) {
    return storage;
  }
  return null;
}

function isStudioLocale(value: string | null | undefined): value is StudioLocale {
  return value === "en" || value === "zh";
}

function localizeTree(root: ParentNode, locale: StudioLocale) {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      const parent = node.parentElement;
      if (!parent || shouldSkipTextElement(parent)) {
        return NodeFilter.FILTER_REJECT;
      }
      return NodeFilter.FILTER_ACCEPT;
    },
  });
  let textNode = walker.nextNode() as Text | null;
  while (textNode) {
    const currentValue = textNode.nodeValue ?? "";
    const previousSource = textSource.get(textNode);
    const source =
      previousSource && isLocalizedValue(currentValue, previousSource)
        ? previousSource
        : currentValue;
    textSource.set(textNode, source);
    const nextValue = locale === "en" ? source : translate(source, locale);
    if (textNode.nodeValue !== nextValue) {
      textNode.nodeValue = nextValue;
    }
    textNode = walker.nextNode() as Text | null;
  }

  if (root instanceof Element) {
    localizeAttributes(root, locale);
  }
  root.querySelectorAll?.("[aria-label], [placeholder], [title]").forEach(
    (element) => localizeAttributes(element, locale),
  );
}

function localizeAttributes(element: Element, locale: StudioLocale) {
  if (shouldSkipAttributeElement(element)) {
    return;
  }
  for (const attr of ["aria-label", "placeholder", "title"]) {
    const value = element.getAttribute(attr);
    if (!value) {
      continue;
    }
    let sourceMap = attributeSource.get(element);
    if (!sourceMap) {
      sourceMap = new Map();
      attributeSource.set(element, sourceMap);
    }
    const previousSource = sourceMap.get(attr);
    const source =
      previousSource && isLocalizedValue(value, previousSource)
        ? previousSource
        : value;
    sourceMap.set(attr, source);
    const nextValue = locale === "en" ? source : translate(source, locale);
    if (value !== nextValue) {
      element.setAttribute(attr, nextValue);
    }
  }
}

function shouldSkipTextElement(element: Element) {
  return ["SCRIPT", "STYLE", "TEXTAREA", "INPUT", "OPTION"].includes(
    element.tagName,
  );
}

function shouldSkipAttributeElement(element: Element) {
  return ["SCRIPT", "STYLE"].includes(
    element.tagName,
  );
}

function isLocalizedValue(value: string, source: string) {
  return supportedLocales.some((candidateLocale) => {
    return value === translate(source, candidateLocale);
  });
}
