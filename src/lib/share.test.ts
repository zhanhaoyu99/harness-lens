import { describe, expect, it } from "vitest";
import { sampleSnapshot } from "./sample";
import { buildSyntheticCompatibilityExample, shareStats } from "./share";

describe("share snapshot", () => {
  it("keeps the synthetic schema-v1 example aggregate-only and unmistakable", () => {
    const summary = buildSyntheticCompatibilityExample(sampleSnapshot);

    expect(summary).toContain("SYNTHETIC DEMO");
    expect(summary).toContain("Report schema: 1");
    expect(summary).toContain("Not compatibility evidence");
    expect(summary).not.toContain(sampleSnapshot.workspacePath);
    expect(summary).not.toContain("Global agreements");
  });

  it("counts diagnostics independently from resolution", () => {
    const stats = shareStats(sampleSnapshot);

    expect(stats.resolved).toBe(4);
    expect(stats.differenceGroups).toBe(1);
  });
});
