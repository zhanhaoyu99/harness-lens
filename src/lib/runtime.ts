import { messages, type Language } from "./i18n";
import type { CodexRunDetail, CodexRunStep } from "../types";

export interface TurnReplay {
  id: string;
  status: string;
  startedAt: string | null;
  completedAt: string | null;
  durationMs: number | null;
  steps: CodexRunStep[];
}

export function buildTurnReplay(detail: CodexRunDetail): TurnReplay[] {
  const stepsByTurn = new Map<string, CodexRunStep[]>();
  for (const step of detail.steps) {
    const turnSteps = stepsByTurn.get(step.turnId) ?? [];
    turnSteps.push(step);
    stepsByTurn.set(step.turnId, turnSteps);
  }

  return detail.turns.map((turn) => ({
    id: turn.id,
    status: turn.status,
    startedAt: turn.startedAt,
    completedAt: turn.completedAt,
    durationMs: turn.durationMs,
    steps: stepsByTurn.get(turn.id) ?? [],
  }));
}

export function formatRuntimeStatus(status: string, language: Language): string {
  const labels = messages[language].runtime.statuses as Record<string, string>;
  return labels[status] ?? status;
}

export function formatStepKind(
  step: Pick<CodexRunStep, "kind" | "label">,
  language: Language,
): string {
  const labels = messages[language].runtime.stepKinds as Record<string, string>;
  return labels[step.kind] ?? step.label;
}

export function formatDuration(durationMs: number | null, language: Language): string {
  if (durationMs === null) return "—";
  const totalSeconds = Math.max(0, Math.round(durationMs / 1_000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (!minutes) return language === "zh" ? `${seconds} 秒` : `${seconds}s`;
  return language === "zh" ? `${minutes} 分 ${seconds} 秒` : `${minutes}m ${seconds}s`;
}

export function formatRuntimeTime(value: string | null, language: Language): string {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "—";
  return new Intl.DateTimeFormat(language === "zh" ? "zh-CN" : "en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}
