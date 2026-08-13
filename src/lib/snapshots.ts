import type {
  ContextSnapshotComparison,
  ContextSnapshotDiagnostic,
  ContextSnapshotItem,
  ContextSnapshotSummary,
  HarnessArtifact,
  HarnessSnapshot,
  SnapshotArtifactChange,
  StoredContextSnapshot,
} from "../types";

export function snapshotIsComplete(snapshot: HarnessSnapshot): boolean {
  return !snapshot.warnings.some(
    (warning) => warning.id === "scan-incomplete" || warning.severity === "error",
  );
}

export function safeStoredSnapshot(
  captureId: string,
  snapshotId: string,
  snapshot: HarnessSnapshot,
  capturedAt = snapshot.scannedAt,
): StoredContextSnapshot {
  const items = snapshot.artifacts.map(safeSnapshotItem);
  const diagnostics: ContextSnapshotDiagnostic[] = snapshot.warnings.map((warning) => ({
    id: warning.id,
    severity: warning.severity,
    artifactIds: warning.artifactIds,
  }));
  const summary: ContextSnapshotSummary = {
    captureId,
    snapshotId,
    schemaVersion: 1,
    workspaceKey: "synthetic-workspace-key",
    workspaceName: snapshot.workspaceName,
    gitBranch: snapshot.gitBranch,
    capturedAt,
    itemCount: items.length,
    diagnosticCount: diagnostics.length,
    complete: snapshotIsComplete(snapshot),
    appVersion: "0.4.0-demo",
    scannerVersion: "1",
  };
  return { summary, items, diagnostics };
}

export function compareStoredSnapshots(
  base: StoredContextSnapshot,
  target: StoredContextSnapshot,
): ContextSnapshotComparison {
  const beforeById = new Map(base.items.map((item) => [item.id, item]));
  const afterById = new Map(target.items.map((item) => [item.id, item]));
  const changes: SnapshotArtifactChange[] = [];
  let unchangedCount = 0;

  for (const before of base.items) {
    const after = afterById.get(before.id);
    if (!after) {
      changes.push(change(before.id, "removed", before, null));
      continue;
    }

    const contentChanged = before.contentHash !== after.contentHash;
    const resolutionChanged = before.resolution !== after.resolution;
    const metadataChanged = itemMetadataChanged(before, after);
    if (contentChanged || resolutionChanged || metadataChanged) {
      changes.push({
        artifactId: before.id,
        kind: "changed",
        before,
        after,
        contentChanged,
        resolutionChanged,
        metadataChanged,
      });
    } else {
      unchangedCount += 1;
    }
  }

  for (const after of target.items) {
    if (!beforeById.has(after.id)) {
      changes.push(change(after.id, "added", null, after));
    }
  }

  changes.sort((left, right) => {
    const order = { added: 0, removed: 1, changed: 2 } as const;
    const kindDifference = order[left.kind] - order[right.kind];
    if (kindDifference !== 0) return kindDifference;
    return changeItem(left).name.localeCompare(changeItem(right).name);
  });

  return {
    base: base.summary,
    target: target.summary,
    changes,
    unchangedCount,
    diagnosticsChanged: normalizedDiagnostics(base) !== normalizedDiagnostics(target),
    complete: base.summary.complete && target.summary.complete,
  };
}

function itemMetadataChanged(
  before: ContextSnapshotItem,
  after: ContextSnapshotItem,
): boolean {
  return before.name !== after.name
    || before.kind !== after.kind
    || before.provider !== after.provider
    || before.scope !== after.scope
    || before.sourceLabel !== after.sourceLabel
    || before.sizeBytes !== after.sizeBytes
    || before.duplicateGroupId !== after.duplicateGroupId
    || before.counterpartId !== after.counterpartId;
}

function normalizedDiagnostics(snapshot: StoredContextSnapshot): string {
  return JSON.stringify(snapshot.diagnostics
    .map((diagnostic) => ({
      ...diagnostic,
      artifactIds: [...diagnostic.artifactIds].sort(),
    }))
    .sort((left, right) => left.id.localeCompare(right.id)));
}

export function changeItem(item: SnapshotArtifactChange): ContextSnapshotItem {
  const artifact = item.after ?? item.before;
  if (!artifact) throw new Error("A snapshot change must reference at least one item.");
  return artifact;
}

function safeSnapshotItem(artifact: HarnessArtifact): ContextSnapshotItem {
  return {
    id: artifact.id,
    name: artifact.name,
    kind: artifact.kind,
    provider: artifact.provider,
    scope: artifact.scope,
    sourceLabel: `${artifact.provider} · ${artifact.scope}`,
    contentHash: artifact.contentHash,
    sizeBytes: artifact.sizeBytes,
    resolution: artifact.resolution,
    duplicateGroupId: artifact.duplicateGroupId,
    counterpartId: artifact.counterpartId,
  };
}

function change(
  artifactId: string,
  kind: "added" | "removed",
  before: ContextSnapshotItem | null,
  after: ContextSnapshotItem | null,
): SnapshotArtifactChange {
  return {
    artifactId,
    kind,
    before,
    after,
    contentChanged: false,
    resolutionChanged: false,
    metadataChanged: false,
  };
}
