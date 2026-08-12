import { counterpartDifferenceCount, effectiveCount } from "./artifacts";
import { messages, type Language } from "./i18n";
import type { HarnessKind, HarnessSnapshot } from "../types";

export interface ShareStats {
  resolved: number;
  differenceGroups: number;
  duplicateGroups: number;
  unknown: number;
  byKind: Array<{ kind: HarnessKind; count: number }>;
}

export function shareStats(snapshot: HarnessSnapshot): ShareStats {
  const kindCounts = new Map<HarnessKind, number>();
  const duplicateGroups = new Set<string>();
  for (const artifact of snapshot.artifacts) {
    kindCounts.set(artifact.kind, (kindCounts.get(artifact.kind) ?? 0) + 1);
    if (artifact.duplicateGroupId) duplicateGroups.add(artifact.duplicateGroupId);
  }

  return {
    resolved: effectiveCount(snapshot.artifacts),
    differenceGroups: counterpartDifferenceCount(snapshot),
    duplicateGroups: duplicateGroups.size,
    unknown: snapshot.artifacts.filter((item) => item.resolution === "unknown").length,
    byKind: Array.from(kindCounts, ([kind, count]) => ({ kind, count })).sort(
      (left, right) => right.count - left.count,
    ),
  };
}

export function buildShareSummary(
  snapshot: HarnessSnapshot,
  language: Language,
): string {
  const copy = messages[language];
  const share = copy.shareSnapshot;
  const stats = shareStats(snapshot);
  const kindLines = stats.byKind
    .map(({ kind, count }) => `- ${copy.labels.kind[kind]}: ${count}`)
    .join("\n");

  return [
    `# ${share.title} — ${snapshot.workspaceName}`,
    "",
    `${share.inventory}: ${snapshot.artifacts.length}`,
    `${share.resolved}: ${stats.resolved}`,
    `${share.drift}: ${stats.differenceGroups}`,
    `${share.duplicates}: ${stats.duplicateGroups}`,
    `${share.unknown}: ${stats.unknown}`,
    "",
    `## ${share.byType}`,
    kindLines,
    "",
    `_${share.privacy}_`,
  ].join("\n");
}
