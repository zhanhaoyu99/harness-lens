import type { HarnessArtifact, HarnessKind, HarnessProvider } from "../types";

export interface ArtifactFilter {
  provider?: HarnessProvider;
  kind?: HarnessKind;
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

export function effectiveCount(artifacts: HarnessArtifact[]): number {
  return artifacts.filter((artifact) => artifact.resolution === "effective").length;
}

export function driftCount(artifacts: HarnessArtifact[]): number {
  return artifacts.filter((artifact) => artifact.counterpartId !== null).length;
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
