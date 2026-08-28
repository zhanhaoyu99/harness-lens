import { describe, expect, it } from "vitest";
import {
  artifactDiagnostics,
  artifactSummary,
  artifactSummarySource,
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

  it("uses declared descriptions without promoting arbitrary file content", () => {
    const skill = sampleSnapshot.artifacts.find(
      (item) => item.id === "skill-verify",
    );
    const projectInstructions = sampleSnapshot.artifacts.find(
      (item) => item.id === "rule-repo",
    );

    expect(skill).toBeDefined();
    expect(projectInstructions).toBeDefined();
    expect(artifactSummary(skill!)).toContain("device acceptance");
    expect(artifactSummarySource(skill!)).toBe("declared");
    expect(artifactSummary(projectInstructions!)).toContain("Guides agent behavior");
    expect(artifactSummary(projectInstructions!)).not.toContain("Project guidance");
    expect(artifactSummarySource(projectInstructions!)).toBe("generated");
  });

  it("associates and prioritizes deterministic diagnostics for one artifact", () => {
    const artifact = {
      ...sampleSnapshot.artifacts.find((item) => item.id === "rule-repo")!,
      lineCount: 237,
    };
    const diagnostics = artifactDiagnostics(artifact, [
      {
        id: "quality:guidance-line-review",
        severity: "info",
        title: "Long guidance",
        detail: "Review it",
        artifactIds: [artifact.id],
      },
      {
        id: "quality:codex-project-instruction-budget",
        severity: "warning",
        title: "Instruction budget",
        detail: "Review it",
        artifactIds: [artifact.id],
      },
      {
        id: "quality:empty-definition",
        severity: "warning",
        title: "Other artifact",
        detail: "Ignore it",
        artifactIds: ["another-id"],
      },
    ]);

    expect(diagnostics.map((item) => item.code)).toEqual([
      "codexProjectInstructionBudget",
      "guidanceLineReview",
    ]);
  });
});
