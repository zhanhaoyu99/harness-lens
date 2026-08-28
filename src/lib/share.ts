import { counterpartDifferenceCount, effectiveCount } from "./artifacts";
import type {
  AggregateCompatibilityReport,
  HarnessKind,
  HarnessSnapshot,
} from "../types";

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

function countBy<T extends string>(values: T[]): Record<string, number> {
  return values.reduce<Record<string, number>>((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
}

function appendCounts(
  lines: string[],
  heading: string,
  counts: Record<string, number>,
): void {
  lines.push("", `## ${heading}`, "");
  const entries = Object.entries(counts).sort(([left], [right]) => left.localeCompare(right));
  if (!entries.length) {
    lines.push("- None");
    return;
  }
  lines.push(...entries.map(([label, count]) => `- ${label}: ${count}`));
}

function formatSyntheticReport(report: AggregateCompatibilityReport): string {
  const lines = [
    "# Harness Lens compatibility report",
    "",
    "> SYNTHETIC DEMO — schema v1 example only. Not compatibility evidence.",
    "",
    `- Report schema: ${report.reportSchemaVersion}`,
    `- Harness Lens: ${report.harnessLensVersion}`,
    "- Source revision: synthetic",
    "- Source dirty: unknown",
    `- Platform: ${report.operatingSystem} / ${report.architecture}`,
    `- Scan complete: ${report.scanComplete ? "yes" : "no"}`,
    `- Artifacts discovered: ${report.artifactCount}`,
    `- Warnings: ${report.warningCounts.info} info / ${report.warningCounts.warning} warning / ${report.warningCounts.error} error`,
  ];
  appendCounts(lines, "By provider", report.byProvider);
  appendCounts(lines, "By kind", report.byKind);
  appendCounts(lines, "By resolution", report.byResolution);
  lines.push(
    "",
    "## Evidence boundary",
    "",
    "This synthetic example demonstrates the report format only. It does not describe a real workspace or provide compatibility evidence.",
    "",
    `_${report.privacyNotice}_`,
  );
  return lines.join("\n");
}

export function buildSyntheticCompatibilityExample(snapshot: HarnessSnapshot): string {
  const warningCounts = {
    info: snapshot.warnings.filter((warning) => warning.severity === "info").length,
    warning: snapshot.warnings.filter((warning) => warning.severity === "warning").length,
    error: snapshot.warnings.filter((warning) => warning.severity === "error").length,
  };
  const report: AggregateCompatibilityReport = {
    reportSchemaVersion: 1,
    harnessLensVersion: "0.5.0-demo",
    sourceRevision: null,
    sourceDirty: null,
    operatingSystem: "browser-demo",
    architecture: "synthetic",
    artifactCount: snapshot.artifacts.length,
    byProvider: countBy(snapshot.artifacts.map((artifact) => artifact.provider)),
    byKind: countBy(snapshot.artifacts.map((artifact) => artifact.kind)),
    byResolution: countBy(snapshot.artifacts.map((artifact) => artifact.resolution)),
    warningCounts,
    scanComplete: warningCounts.error === 0
      && !snapshot.warnings.some((warning) => warning.id === "scan-incomplete"),
    privacyNotice: "Synthetic aggregate metadata only. Review before sharing.",
  };
  return formatSyntheticReport(report);
}
