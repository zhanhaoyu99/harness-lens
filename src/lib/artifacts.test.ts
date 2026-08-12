import { describe, expect, it } from "vitest";
import {
  artifactSummary,
  counterpartDifferenceCount,
  effectiveCount,
  filterArtifacts,
  providerFacetCounts,
} from "./artifacts";
import { sampleSnapshot } from "./sample";

describe("artifact helpers", () => {
  it("filters by provider, kind and content search", () => {
    const results = filterArtifacts(sampleSnapshot.artifacts, {
      provider: "codex",
      kind: "instructions",
      scope: "repo",
      search: "project guidance",
    });

    expect(results.map((item) => item.id)).toEqual(["rule-repo"]);
  });

  it("counts provider facets after applying every non-provider filter", () => {
    const counts = providerFacetCounts(sampleSnapshot.artifacts, {
      provider: "codex",
      kind: "agent",
      scope: "repo",
      search: "qa",
    });

    expect(counts).toEqual({
      total: 2,
      byProvider: {
        codex: 1,
        claude: 1,
        shared: 0,
        plugin: 0,
      },
    });
  });

  it("keeps evidence states separate", () => {
    expect(effectiveCount(sampleSnapshot.artifacts)).toBe(4);
    expect(counterpartDifferenceCount(sampleSnapshot)).toBe(1);
  });

  it("counts backend difference groups without collapsing project layers", () => {
    const nestedDifferences = {
      ...sampleSnapshot,
      warnings: [
        {
          id: "counterpart-difference:Nested:parent:Skill:qa",
          severity: "info" as const,
          title: "Parent difference",
          detail: "Parent layer",
          artifactIds: ["parent-codex", "parent-claude"],
        },
        {
          id: "counterpart-difference:Nested:child:Skill:qa",
          severity: "info" as const,
          title: "Child difference",
          detail: "Child layer",
          artifactIds: ["child-codex", "child-claude"],
        },
      ],
    };

    expect(counterpartDifferenceCount(nestedDifferences)).toBe(2);
  });

  it("surfaces a concise content summary", () => {
    const projectInstructions = sampleSnapshot.artifacts.find(
      (item) => item.id === "rule-repo",
    );

    expect(projectInstructions).toBeDefined();
    expect(artifactSummary(projectInstructions!)).toContain("Project guidance");
  });
});
