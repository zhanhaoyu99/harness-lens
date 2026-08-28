import type {
  HarnessArtifact,
  HarnessKind,
  HarnessProvider,
  HarnessScope,
  HarnessSnapshot,
  HarnessWarning,
} from "../types";

export interface ArtifactFilter {
  provider?: HarnessProvider;
  kind?: HarnessKind;
  scope?: HarnessScope;
  search?: string;
}

export function filterArtifacts(
  artifacts: HarnessArtifact[],
  filter: ArtifactFilter,
): HarnessArtifact[] {
  const query = filter.search?.trim().toLocaleLowerCase();
  return artifacts.filter((artifact) => {
    if (filter.provider && artifact.provider !== filter.provider) return false;
    if (filter.kind && artifact.kind !== filter.kind) return false;
    if (filter.scope && artifact.scope !== filter.scope) return false;
    if (!query) return true;
    return [
      artifact.name,
      artifact.description,
      artifact.relativePath,
      artifact.path,
      artifact.content,
    ].some((value) => value?.toLocaleLowerCase().includes(query));
  });
}

export interface ProviderFacetCounts {
  total: number;
  byProvider: Record<HarnessProvider, number>;
}

export function providerFacetCounts(
  artifacts: HarnessArtifact[],
  filter: ArtifactFilter,
): ProviderFacetCounts {
  const matchingArtifacts = filterArtifacts(artifacts, {
    ...filter,
    provider: undefined,
  });
  const byProvider: Record<HarnessProvider, number> = {
    codex: 0,
    claude: 0,
    shared: 0,
    plugin: 0,
  };

  for (const artifact of matchingArtifacts) {
    byProvider[artifact.provider] += 1;
  }

  return {
    total: matchingArtifacts.length,
    byProvider,
  };
}

export function effectiveCount(artifacts: HarnessArtifact[]): number {
  return artifacts.filter((artifact) => artifact.resolution === "effective").length;
}

export function isCounterpartDifferenceWarning(warning: HarnessWarning): boolean {
  return warning.id.startsWith("counterpart-difference:")
    || warning.id.startsWith("cross-provider-difference:");
}

export function counterpartDifferenceCount(
  snapshot: Pick<HarnessSnapshot, "artifacts" | "warnings">,
  provider?: HarnessProvider,
): number {
  if (!provider) {
    return snapshot.warnings.filter(isCounterpartDifferenceWarning).length;
  }

  const providerArtifactIds = new Set(
    snapshot.artifacts
      .filter((artifact) => artifact.provider === provider)
      .map((artifact) => artifact.id),
  );
  return snapshot.warnings.filter(
    (warning) => isCounterpartDifferenceWarning(warning)
      && warning.artifactIds.some((artifactId) => providerArtifactIds.has(artifactId)),
  ).length;
}

export interface ArtifactSummaryFallbacks {
  byKind: Record<HarnessKind, string>;
}

const defaultSummaryFallbacks: ArtifactSummaryFallbacks = {
  byKind: {
    instructions: "Guides agent behavior at this scope; applicability still follows provider precedence.",
    skill: "Provides reusable agent instructions or a workflow; runtime use is not inferred.",
    hook: "Defines lifecycle behavior; execution requires runtime evidence.",
    agent: "Defines a specialized agent profile; runtime registration is not assumed.",
    config: "Configures provider behavior; effective values still depend on trust and precedence.",
    memory: "Stores maintained context; content is loaded only on request.",
    rule: "Constrains or permits agent behavior; enforcement depends on provider resolution.",
    workflow: "Describes a reusable multi-step process; executability is not assumed.",
    plugin: "Contributes packaged Harness capabilities; installation does not prove activation.",
  },
};

export function artifactSummary(
  artifact: HarnessArtifact,
  fallbacks: ArtifactSummaryFallbacks = defaultSummaryFallbacks,
): string {
  if (artifact.description?.trim()) return artifact.description.trim();
  return fallbacks.byKind[artifact.kind];
}

export type ArtifactSummarySource = "declared" | "generated";

export function artifactSummarySource(
  artifact: HarnessArtifact,
): ArtifactSummarySource {
  return artifact.description?.trim() ? "declared" : "generated";
}

export type ArtifactDiagnosticCode =
  | "guidanceLineReview"
  | "skillDescriptionMissing"
  | "emptyDefinition"
  | "previewTruncated"
  | "codexProjectInstructionBudget"
  | "duplicate"
  | "counterpartDifference"
  | "other";

export interface ArtifactDiagnostic {
  code: ArtifactDiagnosticCode;
  warning: HarnessWarning;
}

export function artifactDiagnostics(
  artifact: HarnessArtifact,
  warnings: HarnessWarning[],
): ArtifactDiagnostic[] {
  const severityOrder: Record<HarnessWarning["severity"], number> = {
    error: 0,
    warning: 1,
    info: 2,
  };

  return warnings
    .filter((warning) => warning.artifactIds.includes(artifact.id))
    .map((warning) => ({
      warning,
      code: diagnosticCode(warning),
    }))
    .sort((left, right) => {
      const severityDifference = severityOrder[left.warning.severity]
        - severityOrder[right.warning.severity];
      if (severityDifference !== 0) return severityDifference;
      return left.warning.id.localeCompare(right.warning.id);
    });
}

function diagnosticCode(warning: HarnessWarning): ArtifactDiagnosticCode {
  switch (warning.id) {
    case "quality:guidance-line-review":
      return "guidanceLineReview";
    case "quality:skill-description-missing":
      return "skillDescriptionMissing";
    case "quality:empty-definition":
      return "emptyDefinition";
    case "quality:preview-truncated":
      return "previewTruncated";
    case "quality:codex-project-instruction-budget":
      return "codexProjectInstructionBudget";
    default:
      if (warning.id.startsWith("duplicate:")) return "duplicate";
      if (isCounterpartDifferenceWarning(warning)) return "counterpartDifference";
      return "other";
  }
}
