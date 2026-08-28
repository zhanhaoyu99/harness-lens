import {
  AlertCircle,
  CheckCircle2,
  ExternalLink,
  FileQuestion,
  GitCompareArrows,
  LoaderCircle,
  RefreshCw,
  RotateCcw,
  Save,
  ShieldCheck,
} from "lucide-react";
import {
  artifactDiagnostics,
  artifactSummary,
  artifactSummarySource,
} from "../lib/artifacts";
import {
  localizeArtifactDiagnostic,
  localizeResolutionReason,
  messages,
  type Language,
} from "../lib/i18n";
import { formatBytes, shortPath } from "../lib/labels";
import type {
  HarnessArtifact,
  HarnessWarning,
  MemoryArtifactDocument,
} from "../types";

export interface LoadedMemoryState {
  document: MemoryArtifactDocument;
  draft: string;
}

interface InspectorProps {
  artifact: HarnessArtifact | null;
  counterpart: HarnessArtifact | null;
  warnings: HarnessWarning[];
  language: Language;
  workspacePath: string | null;
  groupedArtifacts: HarnessArtifact[];
  loadedMemory: LoadedMemoryState | null;
  memoryLoading: boolean;
  memorySaving: boolean;
  memoryError: string | null;
  memoryFeedback: "saved" | "savedRefreshFailed" | "cancelled" | "discarded" | null;
  canLoadMemory: boolean;
  onSelect: (id: string) => void;
  onOpenSource: (path: string) => void;
  onLoadMemory: () => void;
  onReloadMemory: () => void;
  onChangeMemoryDraft: (content: string) => void;
  onCancelMemoryChanges: () => void;
  onSaveMemory: () => void;
}

export function Inspector({
  artifact,
  counterpart,
  warnings,
  language,
  workspacePath,
  groupedArtifacts,
  loadedMemory,
  memoryLoading,
  memorySaving,
  memoryError,
  memoryFeedback,
  canLoadMemory,
  onSelect,
  onOpenSource,
  onLoadMemory,
  onReloadMemory,
  onChangeMemoryDraft,
  onCancelMemoryChanges,
  onSaveMemory,
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
                  <small>
                    {copy.labels.provider[item.provider]} · {copy.labels.scope[item.scope]} · {copy.labels.resolution[item.resolution]}
                  </small>
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

  const memory = loadedMemory?.document.artifactId === artifact.id
    ? loadedMemory
    : null;
  const memoryDirty = memory ? memory.draft !== memory.document.content : false;
  const purpose = artifactSummary(artifact, {
    byKind: copy.artifact.purposeByKind,
  });
  const purposeSource = artifactSummarySource(artifact) === "declared"
    ? copy.artifact.declaredPurpose
    : copy.artifact.generatedPurpose;
  const diagnostics = artifactDiagnostics(artifact, warnings);

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
      </div>

      <dl className="metadata-grid">
        <div>
          <dt>{copy.inspector.provider}</dt>
          <dd>{copy.labels.provider[artifact.provider]}</dd>
        </div>
        <div>
          <dt>{copy.inspector.scope}</dt>
          <dd><span className={`scope-badge scope-${artifact.scope}`}>{copy.labels.scope[artifact.scope]}</span></dd>
        </div>
        <div>
          <dt>{copy.inspector.size}</dt>
          <dd>{formatBytes(artifact.sizeBytes)}</dd>
        </div>
        <div>
          <dt>{copy.inspector.lines}</dt>
          <dd>{artifact.lineCount.toLocaleString(language === "zh" ? "zh-CN" : "en-US")}</dd>
        </div>
        <div>
          <dt>{copy.inspector.hash}</dt>
          <dd className="mono">{artifact.contentHash.slice(0, 10)}</dd>
        </div>
      </dl>

      <section className="inspector-section purpose-section">
        <div className="section-label-row">
          <span>{copy.artifact.positionAndPurpose}</span>
          <FileQuestion size={15} />
        </div>
        <div className="position-purpose-card">
          <strong>
            {copy.labels.provider[artifact.provider]} · {copy.labels.scope[artifact.scope]} · {copy.labels.kind[artifact.kind]}
          </strong>
          <dl>
            <div>
              <dt>{copy.artifact.purpose}</dt>
              <dd>{purpose}</dd>
            </div>
            <div>
              <dt>{copy.artifact.summaryBasis}</dt>
              <dd>{purposeSource}</dd>
            </div>
          </dl>
        </div>
      </section>

      <section className="inspector-section diagnostic-section">
        <div className="section-label-row">
          <span>{copy.artifact.diagnostics}</span>
          <span className="diagnostic-total">
            {diagnostics.length ? copy.artifact.diagnosticCount(diagnostics.length) : copy.artifact.noDiagnostics}
          </span>
        </div>
        <p className="diagnostic-boundary">{copy.artifact.diagnosticsBoundary}</p>
        {diagnostics.length ? (
          <div className="diagnostic-list">
            {diagnostics.map((diagnostic) => {
              const localized = localizeArtifactDiagnostic(diagnostic, artifact, language);
              return (
                <article
                  className={`diagnostic-card severity-${diagnostic.warning.severity}`}
                  key={diagnostic.warning.id}
                >
                  <div className="diagnostic-card-heading">
                    {diagnostic.code === "counterpartDifference"
                      ? <GitCompareArrows size={15} />
                      : <AlertCircle size={15} />}
                    <strong>{localized.title}</strong>
                  </div>
                  <p>{localized.detail}</p>
                  {localized.recommendation ? (
                    <div className="diagnostic-fact">
                      <span>{copy.artifact.recommendation}</span>
                      <p>{localized.recommendation}</p>
                    </div>
                  ) : null}
                  {localized.basis ? (
                    <div className="diagnostic-fact">
                      <span>{copy.artifact.evidenceBasis}</span>
                      <p>{localized.basis}</p>
                      {localized.sourceUrl ? (
                        <a href={localized.sourceUrl} target="_blank" rel="noreferrer">
                          {copy.artifact.documentedSource} <ExternalLink size={11} />
                        </a>
                      ) : null}
                    </div>
                  ) : null}
                  {diagnostic.code === "counterpartDifference" && counterpart ? (
                    <div className="counterpart-card">
                      <div>
                        <strong>{counterpart.name}</strong>
                        <small>
                          {copy.labels.provider[counterpart.provider]} · {copy.labels.scope[counterpart.scope]}
                        </small>
                        <code>{workspacePath ? shortPath(counterpart.path, workspacePath) : counterpart.path}</code>
                      </div>
                      <button className="secondary-button compact-button" onClick={() => onSelect(counterpart.id)}>
                        {copy.inspector.viewCounterpart}
                      </button>
                    </div>
                  ) : null}
                </article>
              );
            })}
          </div>
        ) : (
          <div className="diagnostic-empty">
            <CheckCircle2 size={16} />
            <span>{copy.artifact.noDiagnosticsDetail}</span>
          </div>
        )}
      </section>

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

      {artifact.kind === "memory" ? (
        <section className="inspector-section content-section memory-section">
          <div className="section-label-row">
            <span>{copy.inspector.memoryContent}</span>
            {memoryDirty ? <span className="unsaved-chip">{copy.inspector.unsavedMemory}</span> : null}
          </div>

          {!canLoadMemory ? (
            <div className="content-locked memory-boundary">
              <ShieldCheck size={18} />
              <span>{copy.inspector.syntheticMemory}</span>
            </div>
          ) : !memory ? (
            <div className="memory-load-panel">
              <div className="content-locked memory-boundary">
                <ShieldCheck size={18} />
                <span>{copy.inspector.metadataOnly}</span>
              </div>
              <button
                className="secondary-button memory-load-button"
                disabled={memoryLoading}
                onClick={onLoadMemory}
              >
                {memoryLoading ? <LoaderCircle className="spin" size={15} /> : <ShieldCheck size={15} />}
                {memoryLoading ? copy.inspector.loadingMemory : copy.inspector.loadMemory}
              </button>
            </div>
          ) : (
            <div className="memory-editor">
              <div className="memory-raw-notice">
                <ShieldCheck size={14} />
                <span>{copy.inspector.rawMemoryNotice}</span>
              </div>
              {memory.document.editable ? (
                <textarea
                  aria-label={copy.inspector.memoryContent}
                  value={memory.draft}
                  onChange={(event) => onChangeMemoryDraft(event.target.value)}
                  readOnly={memorySaving || memoryLoading || !memory.document.editToken}
                  spellCheck={false}
                />
              ) : (
                <pre className="memory-viewer">{memory.document.content}</pre>
              )}
              {!memory.document.editable ? (
                <p className="memory-readonly-reason">
                  {memory.document.editabilityReason || copy.inspector.memoryReadOnly}
                </p>
              ) : (
                <p className="memory-autosave-note">{copy.inspector.memoryNoAutosave}</p>
              )}

              {memoryError ? (
                <div className="memory-feedback error">
                  <AlertCircle size={14} />
                  <span>
                    {memoryError}
                    {memory.document.editable && !memory.document.editToken
                      ? ` ${copy.inspector.memoryReloadRequired}`
                      : ""}
                  </span>
                </div>
              ) : memoryFeedback ? (
                <div className={`memory-feedback ${memoryFeedback}`}>
                  {memoryFeedback === "saved" ? <CheckCircle2 size={14} /> : <AlertCircle size={14} />}
                  {memoryFeedback === "saved"
                    ? copy.inspector.memorySaved
                    : memoryFeedback === "savedRefreshFailed"
                      ? copy.inspector.memorySavedRefreshFailed
                    : memoryFeedback === "discarded"
                      ? copy.inspector.memoryChangesDiscarded
                      : copy.inspector.memoryCancelled}
                </div>
              ) : null}

              <div className="memory-actions">
                <button className="secondary-button" disabled={memoryLoading || memorySaving} onClick={onReloadMemory}>
                  <RefreshCw className={memoryLoading ? "spin" : undefined} size={14} />
                  {copy.inspector.reloadMemory}
                </button>
                {memory.document.editable ? (
                  <>
                    <button
                      className="secondary-button"
                      disabled={!memoryDirty || memorySaving || memoryLoading}
                      onClick={onCancelMemoryChanges}
                    >
                      <RotateCcw size={14} />
                      {copy.inspector.cancelMemory}
                    </button>
                    <button
                      className="primary-button compact-button"
                      disabled={!memoryDirty || memorySaving || memoryLoading || !memory.document.editToken}
                      onClick={onSaveMemory}
                    >
                      {memorySaving ? <LoaderCircle className="spin" size={14} /> : <Save size={14} />}
                      {memorySaving ? copy.inspector.savingMemory : copy.inspector.saveMemory}
                    </button>
                  </>
                ) : null}
              </div>
            </div>
          )}
          {!memory && memoryError ? (
            <div className="memory-feedback error"><AlertCircle size={14} />{memoryError}</div>
          ) : null}
        </section>
      ) : (
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
      )}
    </aside>
  );
}
