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
  contentNotLoaded: string;
  noReadableSummary: string;
}

const defaultSummaryFallbacks: ArtifactSummaryFallbacks = {
  contentNotLoaded: "Content is not loaded by default.",
  noReadableSummary: "No readable summary.",
};

export function artifactSummary(
  artifact: HarnessArtifact,
  fallbacks: ArtifactSummaryFallbacks = defaultSummaryFallbacks,
): string {
  if (artifact.description?.trim()) return artifact.description.trim();
  if (!artifact.content?.trim()) return fallbacks.contentNotLoaded;

  const body = artifact.content.replace(/^---[\s\S]*?---\s*/, "");
  const firstMeaningfulLine = body
    .split("\n")
    .map((line) => line.trim().replace(/^(?:#{1,6}|[-*]>?)\s+/, ""))
    .find(Boolean);

  return firstMeaningfulLine ?? fallbacks.noReadableSummary;
}
