import { describe, expect, it } from "vitest";
import {
  artifactSummary,
  driftCount,
  effectiveCount,
  filterArtifacts,
} from "./artifacts";
import { sampleSnapshot } from "./sample";

describe("artifact helpers", () => {
  it("filters by provider, kind and content search", () => {
    const results = filterArtifacts(sampleSnapshot.artifacts, {
      provider: "codex",
      kind: "instructions",
      search: "project guidance",
    });

    expect(results.map((item) => item.id)).toEqual(["rule-repo"]);
  });

  it("keeps evidence states separate", () => {
    expect(effectiveCount(sampleSnapshot.artifacts)).toBe(4);
    expect(driftCount(sampleSnapshot.artifacts)).toBe(2);
  });

  it("surfaces a concise content summary", () => {
    const projectInstructions = sampleSnapshot.artifacts.find(
      (item) => item.id === "rule-repo",
    );

    expect(projectInstructions).toBeDefined();
    expect(artifactSummary(projectInstructions!)).toContain("Project guidance");
  });
});
