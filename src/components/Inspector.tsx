import { ExternalLink, FileQuestion, ShieldCheck } from "lucide-react";
import {
  localizeResolutionReason,
  messages,
  type Language,
} from "../lib/i18n";
import { formatBytes, shortPath } from "../lib/labels";
import type { HarnessArtifact } from "../types";

interface InspectorProps {
  artifact: HarnessArtifact | null;
  language: Language;
  workspacePath: string | null;
  groupedArtifacts: HarnessArtifact[];
  onSelect: (id: string) => void;
  onOpenSource: (path: string) => void;
}

export function Inspector({
  artifact,
  language,
  workspacePath,
  groupedArtifacts,
  onSelect,
  onOpenSource,
}: InspectorProps) {
  const copy = messages[language];
  if (!artifact) {
    return (
      <aside className="inspector empty-inspector">
        {groupedArtifacts.length ? (
          <>
            <div className="inspector-heading">
              <span className="eyebrow">{copy.inspector.selectedGroup}</span>
              <h2>{copy.inspector.matchingItems(groupedArtifacts.length)}</h2>
              <p>{copy.inspector.chooseItem}</p>
            </div>
            <div className="grouped-artifacts">
              {groupedArtifacts.map((item) => (
                <button key={item.id} onClick={() => onSelect(item.id)}>
                  <span>{item.name}</span>
                  <small>{copy.labels.provider[item.provider]} · {copy.labels.resolution[item.resolution]}</small>
                </button>
              ))}
            </div>
          </>
        ) : (
          <div className="inspector-placeholder">
            <FileQuestion size={30} />
            <strong>{copy.inspector.selectTitle}</strong>
            <span>{copy.inspector.selectBody}</span>
          </div>
        )}
      </aside>
    );
  }

  return (
    <aside className="inspector">
      <div className="inspector-heading">
        <div className="inspector-title-row">
          <span className="eyebrow">{copy.labels.kind[artifact.kind]}</span>
          <span className={`status-pill status-${artifact.resolution}`}>
            {copy.labels.resolution[artifact.resolution]}
          </span>
        </div>
        <h2>{artifact.name}</h2>
        {artifact.description ? <p>{artifact.description}</p> : null}
      </div>

      <dl className="metadata-grid">
        <div>
          <dt>{copy.inspector.provider}</dt>
          <dd>{copy.labels.provider[artifact.provider]}</dd>
        </div>
        <div>
          <dt>{copy.inspector.scope}</dt>
          <dd>{copy.labels.scope[artifact.scope]}</dd>
        </div>
        <div>
          <dt>{copy.inspector.size}</dt>
          <dd>{formatBytes(artifact.sizeBytes)}</dd>
        </div>
        <div>
          <dt>{copy.inspector.hash}</dt>
          <dd className="mono">{artifact.contentHash.slice(0, 10)}</dd>
        </div>
      </dl>

      <section className="inspector-section">
        <div className="section-label-row">
          <span>{copy.inspector.whyState}</span>
          <ShieldCheck size={15} />
        </div>
        <p className="resolution-reason">{localizeResolutionReason(artifact.resolutionReason, language)}</p>
      </section>

      <section className="inspector-section source-section">
        <div className="section-label-row">
          <span>{copy.inspector.source}</span>
          <button className="icon-button" onClick={() => onOpenSource(artifact.path)} title={copy.inspector.openSource}>
            <ExternalLink size={15} />
          </button>
        </div>
        <code>{workspacePath ? shortPath(artifact.path, workspacePath) : artifact.path}</code>
      </section>

      <section className="inspector-section content-section">
        <div className="section-label-row">
          <span>{copy.inspector.redactedContent}</span>
          {artifact.sensitive ? <span className="sensitive-chip">{copy.inspector.sensitive}</span> : null}
        </div>
        {artifact.content ? (
          <pre>{artifact.content}{artifact.truncated ? `\n\n${copy.inspector.truncated}` : ""}</pre>
        ) : (
          <div className="content-locked">
            <ShieldCheck size={18} />
            <span>{copy.inspector.metadataOnly}</span>
          </div>
        )}
      </section>
    </aside>
  );
}
