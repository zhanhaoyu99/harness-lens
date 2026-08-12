export type HarnessKind =
  | "instructions"
  | "skill"
  | "hook"
  | "agent"
  | "config"
  | "memory"
  | "rule"
  | "workflow"
  | "plugin";

export type HarnessProvider = "codex" | "claude" | "shared" | "plugin";

export type HarnessScope = "user" | "repo" | "nested" | "worktree";

export type ResolutionState =
  | "effective"
  | "defined"
  | "shadowed"
  | "duplicate"
  | "drifted"
  | "installedInactive"
  | "unknown";

export interface HarnessArtifact {
  id: string;
  name: string;
  kind: HarnessKind;
  provider: HarnessProvider;
  scope: HarnessScope;
  path: string;
  relativePath: string;
  content: string | null;
  contentHash: string;
  modifiedAt: string | null;
  sizeBytes: number;
  resolution: ResolutionState;
  resolutionReason: string;
  duplicateGroupId: string | null;
  counterpartId: string | null;
  description: string | null;
  sensitive: boolean;
  truncated: boolean;
}

export interface HarnessWarning {
  id: string;
  severity: "info" | "warning" | "error";
  title: string;
  detail: string;
  artifactIds: string[];
}

export interface HarnessSnapshot {
  workspacePath: string;
  workspaceName: string;
  gitBranch: string | null;
  scannedAt: string;
  artifacts: HarnessArtifact[];
  warnings: HarnessWarning[];
}

export type RuntimeConnectionState = "connected" | "unavailable" | "error";

export interface CodexRuntimeSkill {
  name: string;
  sourceName: string;
  scope: string;
  enabled: boolean;
}

export interface CodexRuntimeHook {
  eventName: string;
  sourceName: string;
  enabled: boolean;
  trustStatus: string | null;
}

export interface CodexRunSummary {
  id: string;
  title: string;
  preview: string;
  status: string;
  source: string;
  createdAt: string | null;
  updatedAt: string | null;
  parentThreadId: string | null;
  gitBranch: string | null;
}

export interface CodexRuntimeSnapshot {
  state: RuntimeConnectionState;
  codexVersion: string | null;
  observedAt: string;
  message: string | null;
  skills: CodexRuntimeSkill[];
  hooks: CodexRuntimeHook[];
  runs: CodexRunSummary[];
}

export interface CodexRunStep {
  id: string;
  turnId: string;
  kind: string;
  label: string;
  status: string | null;
  detail: string | null;
}

export interface CodexTurnSummary {
  id: string;
  status: string;
  startedAt: string | null;
  completedAt: string | null;
  durationMs: number | null;
  stepCount: number;
}

export interface CodexRunDetail {
  id: string;
  title: string;
  status: string;
  turns: CodexTurnSummary[];
  steps: CodexRunStep[];
  itemTypeCounts: Record<string, number>;
  completedTurns: number;
  failedTurns: number;
  truncated: boolean;
}

export type PrimarySection = "overview" | "items" | "runs" | "compare" | "share";

export type ExplorerMode = "map" | "list";
