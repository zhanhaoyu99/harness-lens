import type { HarnessWarning } from "../types";

export type Language = "zh" | "en";

export const LANGUAGE_STORAGE_KEY = "harness-lens.language";

const en = {
  languageSetting: "Language",
  brandSubtitle: "Agent DevTools",
  nav: {
    overview: "Overview",
    items: "Harness Items",
    runs: "Runs",
    compare: "Compare",
    share: "Share",
  },
  common: {
    primaryNavigation: "Primary navigation",
    next: "Next",
    inventory: "Inventory",
    localOnly: "Local only",
    readOnlyVersion: "v0.1 · read-only",
  },
  workspace: {
    label: "Workspace",
    noneSelected: "No workspace selected",
    rescan: "Rescan",
    open: "Open workspace",
    chooseDialogTitle: "Choose a workspace to inspect",
  },
  emptyWorkspace: {
    eyebrow: "Local-first Agent DevTools",
    title: "See what you maintain in your Agent Harness.",
    body: "Choose a repository or worktree to discover Rules, Skills, Hooks, Agents, Config and Memory across user and project scopes.",
    choose: "Choose workspace",
    privacy: "Read-only · local scanning · secrets redacted",
  },
  stages: {
    ariaLabel: "Harness evidence stages",
    defined: "Defined",
    definedDetail: "discovered items",
    resolved: "Resolved",
    resolvedDetail: "from static rules",
    observed: "Observed",
    observedDetail: "historical runtime evidence",
    evaluated: "Evaluated",
    evaluatedDetail: "task outcome not evaluated",
  },
  runtime: {
    eyebrow: "Read-only Codex runtime",
    title: "Run Flight Recorder",
    body: "Replay the normalized path of a historical Codex run without exposing prompts, tool arguments or changed-file paths.",
    refresh: "Refresh runtime",
    synthetic: "Synthetic demo",
    connection: "Connection",
    version: "CLI version",
    connected: "Connected",
    unavailable: "Unavailable",
    error: "Connection error",
    loading: "Loading runtime evidence…",
    loadingRun: "Loading historical run…",
    observedSummary: (runs: number, skills: number, hooks: number) =>
      `${runs} runs · ${skills} enabled skills · ${hooks} enabled hooks`,
    historicalRuns: "Historical runs",
    noRunsTitle: "No historical runs found",
    noRunsBody: "The Codex App Server returned no runs for this exact workspace.",
    unavailableTitle: "Runtime evidence is unavailable",
    unavailableBody: "Static Harness inventory still works. Install or locate the Codex CLI to inspect historical run metadata.",
    chooseTitle: "Choose a historical run",
    chooseBody: "Run details are loaded only after you select one from the list.",
    turns: (count: number) => `${count} turn${count === 1 ? "" : "s"}`,
    evidenceSteps: (count: number) => `${count} evidence step${count === 1 ? "" : "s"}`,
    turn: (index: number) => `Turn ${index}`,
    noSteps: "No normalized evidence steps in this turn.",
    truncated: "Replay capped at 1,000 evidence steps. Later steps are not shown.",
    evidenceInspector: "Evidence inspector",
    selectEvidence: "Select a step in the linear replay to inspect its safe metadata.",
    kind: "Type",
    status: "Runtime state",
    source: "Source",
    branch: "Branch",
    updated: "Updated",
    duration: "Duration",
    turnId: "Turn",
    detail: "Safe detail",
    metadataOnly: "Metadata only — raw prompts, reasoning, tool arguments and file paths are not returned to this view.",
    boundariesAria: "Evidence boundaries",
    boundaries: {
      historical: "Historical run loaded",
      context: "Harness context not captured",
      outcome: "Task outcome not evaluated",
    },
    boundariesDetail: {
      historical: "This is a read-only replay of an existing thread.",
      context: "Historical App Server data cannot prove which Harness snapshot was active.",
      outcome: "A completed turn is runtime state, not evidence that the task succeeded.",
    },
    statuses: {
      completed: "Completed",
      failed: "Failed",
      interrupted: "Interrupted",
      inProgress: "In progress",
      notLoaded: "Historical",
      unknown: "Unknown",
    },
    stepKinds: {
      userMessage: "User request",
      agentMessage: "Agent message",
      reasoning: "Reasoning",
      commandExecution: "Command execution",
      fileChange: "File changes",
      mcpToolCall: "MCP tool call",
      dynamicToolCall: "Dynamic tool call",
      webSearch: "Web search",
      subAgentActivity: "Subagent activity",
      imageGeneration: "Image generation",
      enteredReviewMode: "Review started",
      exitedReviewMode: "Review completed",
    },
  },
  overview: {
    currentHarness: "Current Harness",
    inventory: "Inventory",
    title: "What do I maintain in this Harness?",
    itemsTitle: "Every maintained Harness item",
    summary: (items: number, providers: number, drift: number) =>
      `${items} items across ${providers} providers. ${
        drift ? `${drift} need attention.` : "No provider drift detected."
      }`,
    snapshot: "Snapshot",
  },
  explorer: {
    map: "Map",
    list: "List",
    search: "Search Harness",
  },
  table: {
    emptyTitle: "No Harness items match this view",
    emptyBody: "Try clearing the search or category filters.",
    inspectAria: (name: string) => `Inspect ${name}`,
    name: "Name",
    kind: "Kind",
    provider: "Provider",
    scope: "Scope",
    status: "Status",
    contentNotLoaded: "Content is not loaded by default.",
    noReadableSummary: "No readable summary.",
  },
  inspector: {
    selectedGroup: "Selected group",
    matchingItems: (count: number) => `${count} matching items`,
    chooseItem: "Choose an item to inspect its source, content and resolution evidence.",
    selectTitle: "Select a node or item",
    selectBody: "The inspector keeps content and resolution evidence one click away.",
    provider: "Provider",
    scope: "Scope",
    size: "Size",
    hash: "Hash",
    whyState: "Why this state",
    source: "Source",
    openSource: "Open source file",
    redactedContent: "Redacted content",
    sensitive: "Sensitive",
    truncated: "… content truncated",
    metadataOnly: "Metadata only. Content was not loaded by default.",
  },
  map: {
    ariaLabel: "Harness topology map",
    harnessItems: (count: number) => `${count} Harness items`,
    local: "local",
    discovered: (count: number) => `${count} discovered`,
    drift: (count: number) => `${count} drift`,
    items: (count: number) => `${count} item${count === 1 ? "" : "s"}`,
    effective: (count: number) => `${count} effective`,
  },
  future: {
    eyebrow: "Planned adapter",
    contract: "Visible now so the MVP stays aligned with the full Control Plane goal.",
    runs: {
      title: "Actual path, not just declared flow",
      body: "The Codex App Server adapter will connect turns, tool calls, hooks, subagents, token usage and evidence to the exact Harness snapshot.",
    },
    compare: {
      title: "Compare outcomes across Harness versions",
      body: "Run completion, verifier results, duration, tokens and failure categories will be compared without treating an Agent's final answer as success.",
    },
    share: {
      title: "Share a safe Flight Recorder",
      body: "Export a redacted Harness Map or static Run Replay after reviewing every included field. No cloud workspace is required.",
    },
  },
  shareSnapshot: {
    eyebrow: "Safe to share",
    title: "Redacted Harness Snapshot",
    body: "Review an aggregate-only card, then copy a Markdown summary for chat, email or documentation.",
    inventory: "Discovered",
    resolved: "Statically resolved",
    drift: "Drifted items",
    duplicates: "Duplicate groups",
    unknown: "Unknown",
    byType: "By type",
    privacy: "No file content or absolute paths included.",
    copy: "Copy summary",
    copied: "Copied",
    copyError: "Copy failed",
  },
  labels: {
    kind: {
      instructions: "Instructions",
      skill: "Skills",
      hook: "Hooks",
      agent: "Agents",
      config: "Config",
      memory: "Memory",
      rule: "Rules",
      workflow: "Workflows",
      plugin: "Plugins",
    },
    provider: {
      codex: "Codex",
      claude: "Claude",
      shared: "Shared",
      plugin: "Plugin",
    },
    scope: {
      user: "User",
      repo: "Repo",
      nested: "Nested",
      worktree: "Worktree",
    },
    resolution: {
      effective: "Effective",
      defined: "Defined",
      shadowed: "Shadowed",
      duplicate: "Duplicate",
      drifted: "Drifted",
      installedInactive: "Inactive",
      unknown: "Unknown",
    },
  },
  warnings: {
    runtimeTitle: "Runtime evidence is not connected yet",
    runtimeDetail: "Defined and resolved states come from static adapter rules. Actual usage requires a runtime event source.",
    duplicateTitle: "Duplicate Harness content",
    duplicateDetail: "Multiple discovered items have identical content.",
    driftTitle: (name?: string) => name ? `Provider drift: ${name}` : "Provider drift detected",
    driftDetail: "Same-name Harness items differ across providers.",
  },
};

export type Messages = typeof en;

const zh: Messages = {
  languageSetting: "语言",
  brandSubtitle: "Agent 开发工具",
  nav: {
    overview: "概览",
    items: "Harness 内容",
    runs: "运行",
    compare: "对比",
    share: "分享",
  },
  common: {
    primaryNavigation: "主导航",
    next: "后续",
    inventory: "内容清单",
    localOnly: "仅在本机",
    readOnlyVersion: "v0.1 · 只读",
  },
  workspace: {
    label: "工作区",
    noneSelected: "尚未选择工作区",
    rescan: "重新扫描",
    open: "打开工作区",
    chooseDialogTitle: "选择要查看的工作区",
  },
  emptyWorkspace: {
    eyebrow: "本地优先的 Agent 开发工具",
    title: "看清你在 Agent Harness 中维护的内容。",
    body: "选择一个仓库或工作树，发现用户级和项目级的规则、Skills、Hooks、Agents、配置与记忆。",
    choose: "选择工作区",
    privacy: "只读 · 本地扫描 · 自动脱敏",
  },
  stages: {
    ariaLabel: "Harness 证据阶段",
    defined: "已发现",
    definedDetail: "扫描到的内容",
    resolved: "已解析",
    resolvedDetail: "基于静态规则",
    observed: "已观测",
    observedDetail: "历史运行时证据",
    evaluated: "已验证",
    evaluatedDetail: "未评估任务结果",
  },
  runtime: {
    eyebrow: "只读 Codex 运行时",
    title: "运行飞行记录器",
    body: "回放 Codex 历史运行经过归一化的实际路径，不暴露提示词、工具参数或变更文件路径。",
    refresh: "刷新运行时",
    synthetic: "合成演示",
    connection: "连接状态",
    version: "CLI 版本",
    connected: "已连接",
    unavailable: "不可用",
    error: "连接出错",
    loading: "正在加载运行时证据…",
    loadingRun: "正在加载历史运行…",
    observedSummary: (runs: number, skills: number, hooks: number) =>
      `${runs} 次运行 · ${skills} 个已启用 Skill · ${hooks} 个已启用 Hook`,
    historicalRuns: "历史运行",
    noRunsTitle: "没有找到历史运行",
    noRunsBody: "Codex App Server 没有返回当前精确工作区下的运行记录。",
    unavailableTitle: "运行时证据当前不可用",
    unavailableBody: "静态 Harness 清单仍可正常使用。安装或指定 Codex CLI 后，即可查看历史运行元数据。",
    chooseTitle: "选择一次历史运行",
    chooseBody: "只有从列表中主动选择后，才会加载该次运行的详情。",
    turns: (count: number) => `${count} 个 Turn`,
    evidenceSteps: (count: number) => `${count} 个证据步骤`,
    turn: (index: number) => `Turn ${index}`,
    noSteps: "这个 Turn 中没有可展示的归一化证据步骤。",
    truncated: "回放最多展示 1,000 个证据步骤，后续步骤未显示。",
    evidenceInspector: "证据检查器",
    selectEvidence: "选择线性回放中的一个步骤，查看其安全元数据。",
    kind: "类型",
    status: "运行时状态",
    source: "来源",
    branch: "分支",
    updated: "更新时间",
    duration: "耗时",
    turnId: "所属 Turn",
    detail: "安全详情",
    metadataOnly: "仅展示元数据——原始提示词、推理内容、工具参数和文件路径不会返回到此视图。",
    boundariesAria: "证据边界",
    boundaries: {
      historical: "已加载历史运行",
      context: "未捕获 Harness 上下文",
      outcome: "未评估任务结果",
    },
    boundariesDetail: {
      historical: "这是对已有线程的只读回放。",
      context: "历史 App Server 数据无法证明当时实际生效的 Harness 快照。",
      outcome: "Turn 完成只是运行时状态，不代表任务已经成功。",
    },
    statuses: {
      completed: "已完成",
      failed: "失败",
      interrupted: "已中断",
      inProgress: "进行中",
      notLoaded: "历史记录",
      unknown: "未知",
    },
    stepKinds: {
      userMessage: "用户请求",
      agentMessage: "Agent 消息",
      reasoning: "推理",
      commandExecution: "命令执行",
      fileChange: "文件变更",
      mcpToolCall: "MCP 工具调用",
      dynamicToolCall: "动态工具调用",
      webSearch: "网页搜索",
      subAgentActivity: "子 Agent 活动",
      imageGeneration: "图像生成",
      enteredReviewMode: "开始审查",
      exitedReviewMode: "完成审查",
    },
  },
  overview: {
    currentHarness: "当前 Harness",
    inventory: "内容清单",
    title: "这个 Harness 里维护了什么？",
    itemsTitle: "所有维护的 Harness 内容",
    summary: (items: number, providers: number, drift: number) =>
      `共发现 ${items} 项内容，来自 ${providers} 个提供方。${
        drift ? `${drift} 项需要关注。` : "未发现提供方差异。"
      }`,
    snapshot: "快照",
  },
  explorer: {
    map: "关系图",
    list: "列表",
    search: "搜索 Harness",
  },
  table: {
    emptyTitle: "没有符合当前条件的 Harness 内容",
    emptyBody: "请尝试清除搜索词或分类筛选。",
    inspectAria: (name: string) => `查看 ${name}`,
    name: "名称",
    kind: "类型",
    provider: "提供方",
    scope: "范围",
    status: "状态",
    contentNotLoaded: "默认未加载内容。",
    noReadableSummary: "没有可读摘要。",
  },
  inspector: {
    selectedGroup: "当前分组",
    matchingItems: (count: number) => `${count} 项匹配内容`,
    chooseItem: "选择一项内容，查看来源、正文和状态依据。",
    selectTitle: "选择一个节点或条目",
    selectBody: "只需一次点击，即可查看内容和状态依据。",
    provider: "提供方",
    scope: "范围",
    size: "大小",
    hash: "哈希",
    whyState: "状态依据",
    source: "来源",
    openSource: "打开源文件",
    redactedContent: "已脱敏内容",
    sensitive: "敏感内容",
    truncated: "… 内容已截断",
    metadataOnly: "当前仅展示元数据，默认未加载正文。",
  },
  map: {
    ariaLabel: "Harness 关系图",
    harnessItems: (count: number) => `${count} 项 Harness 内容`,
    local: "本地",
    discovered: (count: number) => `发现 ${count} 项`,
    drift: (count: number) => `${count} 项不一致`,
    items: (count: number) => `${count} 项内容`,
    effective: (count: number) => `${count} 项有效`,
  },
  future: {
    eyebrow: "计划中的适配器",
    contract: "暂时保留此入口，让 MVP 与完整 Control Plane 方向保持一致。",
    runs: {
      title: "查看实际路径，而不只是声明的流程",
      body: "Codex App Server 适配器会把 Turn、工具调用、Hooks、子 Agent、Token 用量和证据关联到准确的 Harness 快照。",
    },
    compare: {
      title: "对比不同 Harness 版本的结果",
      body: "对比运行完成状态、验证结果、耗时、Token 和失败分类，不把 Agent 的最终回答直接视为成功。",
    },
    share: {
      title: "安全分享 Flight Recorder",
      body: "检查每个字段后，导出脱敏的 Harness 关系图或静态 Run Replay，无需云端工作区。",
    },
  },
  shareSnapshot: {
    eyebrow: "可安全分享",
    title: "脱敏 Harness 快照",
    body: "先检查只包含汇总信息的分享卡，再复制 Markdown 摘要到聊天、邮件或文档。",
    inventory: "已发现",
    resolved: "静态解析",
    drift: "不一致条目",
    duplicates: "重复分组",
    unknown: "未知状态",
    byType: "按类型",
    privacy: "不包含文件正文或绝对路径。",
    copy: "复制摘要",
    copied: "已复制",
    copyError: "复制失败",
  },
  labels: {
    kind: {
      instructions: "指令",
      skill: "Skills",
      hook: "Hooks",
      agent: "Agents",
      config: "配置",
      memory: "记忆",
      rule: "规则",
      workflow: "流程",
      plugin: "插件",
    },
    provider: {
      codex: "Codex",
      claude: "Claude",
      shared: "共享",
      plugin: "插件",
    },
    scope: {
      user: "用户级",
      repo: "仓库级",
      nested: "子目录",
      worktree: "工作树",
    },
    resolution: {
      effective: "有效",
      defined: "已发现",
      shadowed: "被覆盖",
      duplicate: "重复",
      drifted: "不一致",
      installedInactive: "未启用",
      unknown: "未知",
    },
  },
  warnings: {
    runtimeTitle: "尚未连接运行时证据",
    runtimeDetail: "已发现和已解析状态来自静态适配规则；实际使用情况需要运行时事件源。",
    duplicateTitle: "发现重复的 Harness 内容",
    duplicateDetail: "多个已发现条目具有完全相同的内容。",
    driftTitle: (name?: string) => name ? `提供方内容不一致：${name}` : "检测到提供方内容不一致",
    driftDetail: "不同提供方中存在同名但内容不同的 Harness 条目。",
  },
};

export const messages: Record<Language, Messages> = { zh, en };

export function resolveInitialLanguage(
  storedLanguage: string | null,
  systemLanguages: readonly string[],
): Language {
  if (storedLanguage === "zh" || storedLanguage === "en") return storedLanguage;
  return systemLanguages[0]?.toLocaleLowerCase().startsWith("zh") ? "zh" : "en";
}

export function getInitialLanguage(): Language {
  if (typeof window === "undefined") return "en";
  try {
    const systemLanguages = window.navigator.languages?.length
      ? window.navigator.languages
      : [window.navigator.language];
    return resolveInitialLanguage(
      window.localStorage.getItem(LANGUAGE_STORAGE_KEY),
      systemLanguages,
    );
  } catch {
    return resolveInitialLanguage(null, [window.navigator.language]);
  }
}

export function persistLanguage(language: Language): void {
  try {
    window.localStorage.setItem(LANGUAGE_STORAGE_KEY, language);
  } catch {
    // The selected language still applies for this session when storage is unavailable.
  }
}

export function localizeWarning(
  warning: HarnessWarning,
  language: Language,
): Pick<HarnessWarning, "title" | "detail"> {
  const copy = messages[language].warnings;
  if (warning.id === "runtime-not-connected" || warning.id === "observed-not-connected") {
    return { title: copy.runtimeTitle, detail: copy.runtimeDetail };
  }
  if (warning.id.startsWith("duplicate:")) {
    return { title: copy.duplicateTitle, detail: copy.duplicateDetail };
  }
  if (warning.id.startsWith("drift:") || warning.id === "drift-agents") {
    const name = warning.title.startsWith("Provider drift:")
      ? warning.title.slice("Provider drift:".length).trim()
      : undefined;
    return { title: copy.driftTitle(name), detail: copy.driftDetail };
  }
  return { title: warning.title, detail: warning.detail };
}

const zhResolutionReasons: Record<string, string> = {
  "Loaded as the global Codex instruction source.": "作为全局 Codex 指令源加载。",
  "Included after global instructions; project guidance has closer scope.": "在全局指令之后生效；项目指令的作用域更接近当前工作区。",
  "Discovered. Runtime usage has not been observed.": "已发现；尚未观测到运行时使用情况。",
  "Available from the repository skill directory.": "可从仓库级 Skill 目录使用。",
  "Available from the user skill directory.": "可从用户级 Skill 目录使用。",
  "A same-name Claude agent exists with different content.": "存在同名但内容不同的 Claude Agent。",
  "A same-name Codex agent exists with different content.": "存在同名但内容不同的 Codex Agent。",
  "A same-name item exists in another provider with different content.": "其他提供方中存在同名但内容不同的条目。",
  "User hook configuration is enabled.": "用户级 Hook 配置已启用。",
  "Memory metadata only. Expand deliberately when runtime usage is connected.": "当前仅展示记忆元数据；连接运行时使用情况后再按需展开。",
  "Metadata only. Expand deliberately to read memory content.": "当前仅展示元数据；请按需展开后读取记忆内容。",
  "Canonical workflow reference; not assumed executable.": "已发现标准流程引用，但不会假定其可执行。",
  "Workflow reference discovered; it is not assumed to be executable.": "已发现流程引用，但不会假定其可执行。",
  "User Codex configuration participates in the effective config chain.": "用户级 Codex 配置参与有效配置链。",
  "Discovered; live hook status requires the Codex runtime adapter.": "已发现；实时 Hook 状态需要 Codex 运行时适配器。",
  "Discovered in the user Codex rules directory.": "在用户级 Codex 规则目录中发现。",
  "Discovered in the Codex-specific skill directory.": "在 Codex 专用 Skill 目录中发现。",
  "Discovered; actual loading requires Claude runtime evidence.": "已发现；实际加载情况需要 Claude 运行时证据。",
  "Discovered in the user Claude configuration directory.": "在用户级 Claude 配置目录中发现。",
  "Discovered; actual invocation requires Claude runtime evidence.": "已发现；实际调用情况需要 Claude 运行时证据。",
  "Included in the Codex instruction chain for this working directory.": "已加入当前工作目录的 Codex 指令链。",
  "Project config is effective only when the runtime trusts this project.": "只有运行时信任此项目时，项目配置才会生效。",
  "Project hooks require trusted-project and runtime status evidence.": "项目 Hooks 需要项目信任状态和运行时状态证据。",
  "Discovered; effective status requires Claude runtime evidence.": "已发现；有效状态需要 Claude 运行时证据。",
  "Discovered Agent definition; runtime registration is not observed yet.": "已发现 Agent 定义；尚未观测到运行时注册情况。",
};

export function localizeResolutionReason(reason: string, language: Language): string {
  return language === "zh" ? (zhResolutionReasons[reason] ?? reason) : reason;
}
