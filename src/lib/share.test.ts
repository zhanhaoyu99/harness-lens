import { describe, expect, it } from "vitest";
import { sampleSnapshot } from "./sample";
import { buildShareSummary, shareStats } from "./share";

describe("share snapshot", () => {
  it("keeps the summary aggregate-only", () => {
    const summary = buildShareSummary(sampleSnapshot, "zh");

    expect(summary).toContain("Harness 快照");
    expect(summary).not.toContain(sampleSnapshot.workspacePath);
    expect(summary).not.toContain("Global agreements");
  });

  it("counts diagnostics independently from resolution", () => {
    const stats = shareStats(sampleSnapshot);

    expect(stats.resolved).toBe(4);
    expect(stats.driftedItems).toBe(2);
  });
});
