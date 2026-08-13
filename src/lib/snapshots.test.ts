import { describe, expect, it } from "vitest";
import { sampleSnapshot } from "./sample";
import {
  compareStoredSnapshots,
  safeStoredSnapshot,
  snapshotIsComplete,
} from "./snapshots";

describe("context snapshot comparison", () => {
  it("keeps content and resolution changes as independent signals", () => {
    const baseSnapshot = structuredClone(sampleSnapshot);
    const targetSnapshot = structuredClone(sampleSnapshot);
    targetSnapshot.artifacts = targetSnapshot.artifacts
      .filter((artifact) => artifact.id !== "workflow")
      .map((artifact) => artifact.id === "rule-repo"
        ? {
            ...artifact,
            contentHash: "changed-hash",
            resolution: "shadowed" as const,
            sizeBytes: artifact.sizeBytes + 1,
          }
        : artifact);

    const comparison = compareStoredSnapshots(
      safeStoredSnapshot("base-capture", "base", baseSnapshot),
      safeStoredSnapshot("target-capture", "target", targetSnapshot),
    );

    expect(comparison.changes.find((item) => item.artifactId === "rule-repo")).toMatchObject({
      kind: "changed",
      contentChanged: true,
      resolutionChanged: true,
      metadataChanged: true,
    });
    expect(comparison.changes.find((item) => item.artifactId === "workflow")?.kind).toBe(
      "removed",
    );
    expect(comparison.unchangedCount).toBe(sampleSnapshot.artifacts.length - 2);
  });

  it("marks a comparison incomplete when either saved scan was incomplete", () => {
    const incomplete = {
      ...sampleSnapshot,
      warnings: [
        ...sampleSnapshot.warnings,
        {
          id: "scan-incomplete",
          severity: "warning" as const,
          title: "Harness scan was incomplete",
          detail: "One source could not be read.",
          artifactIds: [],
        },
      ],
    };

    expect(snapshotIsComplete(incomplete)).toBe(false);
    expect(compareStoredSnapshots(
      safeStoredSnapshot("base-capture", "base", sampleSnapshot),
      safeStoredSnapshot("target-capture", "target", incomplete),
    )).toMatchObject({
      complete: false,
      diagnosticsChanged: true,
    });
  });

  it("keeps persisted sample data free of paths and content", () => {
    const stored = safeStoredSnapshot("capture", "snapshot", sampleSnapshot);
    const serialized = JSON.stringify(stored);

    expect(serialized).not.toContain(sampleSnapshot.workspacePath);
    expect(serialized).not.toContain("Project guidance");
    expect(stored.items[0]).not.toHaveProperty("path");
    expect(stored.items[0]).not.toHaveProperty("content");
  });
});
