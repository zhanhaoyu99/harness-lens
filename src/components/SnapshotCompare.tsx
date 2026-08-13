import { useEffect, useMemo, useState } from "react";
import clsx from "clsx";
import {
  AlertTriangle,
  ArrowLeftRight,
  Camera,
  CheckCircle2,
  CircleDashed,
  Clock3,
  Database,
  FileSearch,
  Info,
  Trash2,
} from "lucide-react";
import { changeItem } from "../lib/snapshots";
import { messages, type Language } from "../lib/i18n";
import type {
  ContextSnapshotComparison,
  ContextSnapshotSummary,
  HarnessSnapshot,
  SnapshotArtifactChange,
  StoredContextSnapshot,
} from "../types";

type ChangeFilter =
  | "all"
  | "added"
  | "removed"
  | "changed"
  | "content"
  | "resolution"
  | "metadata";

interface SnapshotCompareProps {
  currentSnapshot: HarnessSnapshot;
  history: ContextSnapshotSummary[];
  baseCaptureId: string;
  targetCaptureId: string;
  comparison: ContextSnapshotComparison | null;
  inspectedSnapshot: StoredContextSnapshot | null;
  loadingHistory: boolean;
  capturing: boolean;
  captureDisabled: boolean;
  comparing: boolean;
  loadingSnapshot: boolean;
  clearing: boolean;
  feedback: "captured" | "cleared" | null;
  warning?: string | null;
  error: string | null;
  language: Language;
  synthetic: boolean;
  onCapture: () => void;
  onSelectBase: (captureId: string) => void;
  onSelectTarget: (captureId: string) => void;
  onSwap: () => void;
  onCompare: () => void;
  onInspect: (captureId: string) => void;
  onClear: () => void;
}

export function SnapshotCompare({
  currentSnapshot,
  history,
  baseCaptureId,
  targetCaptureId,
  comparison,
  inspectedSnapshot,
  loadingHistory,
  capturing,
  captureDisabled,
  comparing,
  loadingSnapshot,
  clearing,
  feedback,
  warning,
  error,
  language,
  synthetic,
  onCapture,
  onSelectBase,
  onSelectTarget,
  onSwap,
  onCompare,
  onInspect,
  onClear,
}: SnapshotCompareProps) {
  const [filter, setFilter] = useState<ChangeFilter>("all");
  const copy = messages[language];
  const labels = copy.compare;
  const sortedHistory = useMemo(
    () => [...history].sort((left, right) => right.capturedAt.localeCompare(left.capturedAt)),
    [history],
  );
  const counts = useMemo(() => comparisonCounts(comparison), [comparison]);
  const visibleChanges = useMemo(
    () => comparison?.changes.filter((change) => changeMatchesFilter(change, filter)) ?? [],
    [comparison, filter],
  );
  const canCompare = Boolean(
    baseCaptureId
      && targetCaptureId
      && baseCaptureId !== targetCaptureId
      && !comparing
      && !capturing
      && !clearing,
  );

  useEffect(() => {
    setFilter("all");
  }, [comparison]);

  return (
    <div className="compare-page">
      <header className="compare-heading">
        <div>
          <span className="eyebrow">{labels.eyebrow}</span>
          <div className="compare-title-row">
            <h1>{labels.title}</h1>
            {synthetic ? <span className="synthetic-chip">{labels.synthetic}</span> : null}
          </div>
          <p>{labels.body}</p>
        </div>
        <button
          className="primary-button"
          disabled={captureDisabled || capturing || clearing}
          onClick={onCapture}
        >
          <Camera size={16} />
          {capturing ? labels.capturing : labels.capture}
        </button>
      </header>

      <section className="compare-boundaries" aria-label={labels.safeBoundary}>
        <div>
          <Database size={15} />
          <span><strong>{labels.safeBoundary}</strong></span>
        </div>
        <div>
          <Info size={15} />
          <span><strong>{labels.noRuntimeBoundary}</strong></span>
        </div>
        <div className="live-snapshot-note">
          <CircleDashed size={15} />
          <span>
            <strong>{labels.currentPreview}</strong>
            <small>
              {labels.itemCount(currentSnapshot.artifacts.length)} · {formatDate(currentSnapshot.scannedAt, language)}
            </small>
            <small>{labels.currentPreviewBody}</small>
          </span>
        </div>
      </section>

      {feedback ? (
        <div className="compare-feedback" role="status">
          <CheckCircle2 size={15} />
          {feedback === "captured" ? labels.captured : labels.cleared}
        </div>
      ) : null}
      {warning ? (
        <div className="compare-warning" role="status">
          <AlertTriangle size={15} /> {warning}
        </div>
      ) : null}
      {error ? (
        <div className="compare-error" role="alert">
          <AlertTriangle size={15} /> {error || labels.error}
        </div>
      ) : null}

      <div className="compare-workbench">
        <section className="snapshot-history-panel">
          <header className="compare-panel-heading">
            <div>
              <span>{labels.history}</span>
              <small>{labels.historyCount(sortedHistory.length)}</small>
            </div>
            <button
              className="text-button danger"
              disabled={(!sortedHistory.length && !error) || clearing || capturing || comparing || loadingHistory}
              onClick={onClear}
            >
              <Trash2 size={13} /> {labels.clearHistory}
            </button>
          </header>

          {loadingHistory ? (
            <ComparePlaceholder icon={Clock3} title={labels.loadingSnapshot} />
          ) : !sortedHistory.length ? (
            <ComparePlaceholder
              icon={Database}
              title={labels.noHistoryTitle}
              body={labels.noHistoryBody}
            />
          ) : (
            <div className="snapshot-history-list">
              {sortedHistory.map((item) => (
                <button
                  key={item.captureId}
                  className={clsx(
                    inspectedSnapshot?.summary.captureId === item.captureId && "selected",
                  )}
                  aria-label={`${labels.inspect}: ${formatDate(item.capturedAt, language)} · ${item.captureId.slice(0, 8)}`}
                  aria-pressed={inspectedSnapshot?.summary.captureId === item.captureId}
                  onClick={() => onInspect(item.captureId)}
                >
                  <span className="snapshot-history-icon">
                    {item.complete ? <CheckCircle2 size={14} /> : <AlertTriangle size={14} />}
                  </span>
                  <span className="snapshot-history-copy">
                    <strong>{formatDate(item.capturedAt, language)}</strong>
                    <small>
                      {item.gitBranch ?? "—"} · {labels.itemCount(item.itemCount)}
                    </small>
                    <small>
                      {item.complete ? labels.complete : labels.incomplete} · {item.appVersion}
                    </small>
                    <small>{item.captureId.slice(0, 8)} · {item.snapshotId.slice(0, 8)}</small>
                  </span>
                </button>
              ))}
            </div>
          )}

          <SnapshotMetadata
            snapshot={inspectedSnapshot}
            loading={loadingSnapshot}
            language={language}
          />
        </section>

        <section className="snapshot-compare-panel">
          <div className="snapshot-pair-controls">
            <SnapshotSelect
              label={labels.baseline}
              value={baseCaptureId}
              history={sortedHistory}
              language={language}
              placeholder={labels.selectSnapshot}
              onChange={onSelectBase}
            />
            <button
              className="swap-button"
              aria-label={labels.swap}
              title={labels.swap}
              disabled={!baseCaptureId || !targetCaptureId}
              onClick={onSwap}
            >
              <ArrowLeftRight size={15} />
            </button>
            <SnapshotSelect
              label={labels.target}
              value={targetCaptureId}
              history={sortedHistory}
              language={language}
              placeholder={labels.selectSnapshot}
              onChange={onSelectTarget}
            />
            <button className="primary-button compare-button" disabled={!canCompare} onClick={onCompare}>
              <FileSearch size={15} />
              {comparing ? labels.comparing : labels.compareSaved}
            </button>
          </div>

          {sortedHistory.length < 2 ? (
            <ComparePlaceholder
              icon={Camera}
              title={sortedHistory.length ? labels.needTwoTitle : labels.noHistoryTitle}
              body={sortedHistory.length ? labels.needTwoBody : labels.noHistoryBody}
            />
          ) : !comparison ? (
            <ComparePlaceholder
              icon={ArrowLeftRight}
              title={labels.choosePairTitle}
              body={labels.choosePairBody}
            />
          ) : (
            <>
              {!comparison.complete ? (
                <div className="comparison-boundary warning">
                  <AlertTriangle size={15} /> {labels.incompleteBoundary}
                </div>
              ) : null}

              <div className="comparison-summary" aria-label={labels.summary}>
                <Metric label={labels.totalChanges} value={comparison.changes.length} primary />
                <Metric
                  label={comparison.complete ? labels.added : labels.onlyInTarget}
                  value={counts.added}
                />
                <Metric
                  label={comparison.complete ? labels.removed : labels.onlyInBaseline}
                  value={counts.removed}
                />
                <Metric label={labels.changed} value={counts.changed} />
                <Metric label={labels.contentChanged} value={counts.content} />
                <Metric label={labels.resolutionChanged} value={counts.resolution} />
                <Metric label={labels.metadataChanged} value={counts.metadata} />
                <Metric label={labels.unchanged} value={comparison.unchangedCount} />
              </div>
              <div className="comparison-footnotes">
                <span><Info size={13} />{labels.overlappingCounts}</span>
                <span className={comparison.diagnosticsChanged ? "changed" : undefined}>
                  {comparison.diagnosticsChanged
                    ? <AlertTriangle size={13} />
                    : <CheckCircle2 size={13} />}
                  {comparison.diagnosticsChanged
                    ? labels.diagnosticsChanged
                    : labels.diagnosticsStable}
                </span>
              </div>

              <div className="change-filter" role="group" aria-label={labels.filtersAria}>
                {changeFilters(
                  labels,
                  counts,
                  comparison.changes.length,
                  comparison.complete,
                ).map((option) => (
                  <button
                    key={option.id}
                    className={filter === option.id ? "active" : undefined}
                    aria-pressed={filter === option.id}
                    onClick={() => setFilter(option.id)}
                  >
                    {option.label}<small>{option.count}</small>
                  </button>
                ))}
              </div>

              {!visibleChanges.length ? (
                <ComparePlaceholder
                  icon={CheckCircle2}
                  title={comparison.changes.length
                    ? labels.noFilteredChangesTitle
                    : labels.noChangesTitle}
                  body={comparison.changes.length
                    ? labels.noFilteredChangesBody
                    : labels.noChangesBody}
                />
              ) : (
                <div className="snapshot-change-list">
                  {visibleChanges.map((item) => (
                    <SnapshotChangeRow
                      key={`${item.kind}:${item.artifactId}`}
                      change={item}
                      complete={comparison.complete}
                      language={language}
                    />
                  ))}
                </div>
              )}
            </>
          )}
        </section>
      </div>
    </div>
  );
}

function SnapshotSelect({
  label,
  value,
  history,
  language,
  placeholder,
  onChange,
}: {
  label: string;
  value: string;
  history: ContextSnapshotSummary[];
  language: Language;
  placeholder: string;
  onChange: (captureId: string) => void;
}) {
  return (
    <label className="snapshot-select">
      <span>{label}</span>
      <select value={value} aria-label={label} onChange={(event) => onChange(event.target.value)}>
        <option value="">{placeholder}</option>
        {history.map((item) => (
          <option key={item.captureId} value={item.captureId}>
            {formatDate(item.capturedAt, language)} · {item.gitBranch ?? "—"} · {item.captureId.slice(0, 8)}
          </option>
        ))}
      </select>
    </label>
  );
}

function SnapshotMetadata({
  snapshot,
  loading,
  language,
}: {
  snapshot: StoredContextSnapshot | null;
  loading: boolean;
  language: Language;
}) {
  const copy = messages[language];
  const labels = copy.compare;
  if (loading) return <div className="snapshot-metadata-loading">{labels.loadingSnapshot}</div>;
  if (!snapshot) return null;

  return (
    <div className="snapshot-metadata">
      <span className="eyebrow">{labels.selectedSnapshot}</span>
      <dl>
        <div><dt>{labels.schema}</dt><dd>v{snapshot.summary.schemaVersion}</dd></div>
        <div><dt>{labels.appVersion}</dt><dd>{snapshot.summary.appVersion}</dd></div>
        <div><dt>{labels.scannerVersion}</dt><dd>{snapshot.summary.scannerVersion}</dd></div>
        <div>
          <dt>{labels.diagnosticCount(snapshot.summary.diagnosticCount)}</dt>
          <dd>{snapshot.summary.complete ? labels.complete : labels.incomplete}</dd>
        </div>
      </dl>
      <div className="snapshot-item-preview">
        {snapshot.items.slice(0, 5).map((item) => (
          <div key={item.id}>
            <strong>{item.name}</strong>
            <small>
              {copy.labels.provider[item.provider]} · {copy.labels.kind[item.kind]} · {item.sourceLabel}
            </small>
          </div>
        ))}
        {snapshot.items.length > 5 ? <small>+{snapshot.items.length - 5}</small> : null}
      </div>
    </div>
  );
}

function SnapshotChangeRow({
  change,
  complete,
  language,
}: {
  change: SnapshotArtifactChange;
  complete: boolean;
  language: Language;
}) {
  const copy = messages[language];
  const labels = copy.compare;
  const item = changeItem(change);
  const changeLabel = change.kind === "added"
    ? (complete ? labels.added : labels.onlyInTarget)
    : change.kind === "removed"
      ? (complete ? labels.removed : labels.onlyInBaseline)
      : labels.changed;

  return (
    <article className={`snapshot-change snapshot-change-${change.kind}`}>
      <header>
        <div>
          <span className="change-kind">{changeLabel}</span>
          <strong>{item.name}</strong>
          <small>
            {copy.labels.provider[item.provider]} · {copy.labels.kind[item.kind]} · {copy.labels.scope[item.scope]}
          </small>
        </div>
        <div className="change-signals">
          {change.contentChanged ? <span>{labels.contentChanged}</span> : null}
          {change.resolutionChanged ? <span>{labels.resolutionChanged}</span> : null}
          {change.metadataChanged ? <span>{labels.metadataChanged}</span> : null}
        </div>
      </header>
      <div className="snapshot-change-sides">
        <SnapshotSide label={labels.before} item={change.before} language={language} />
        <SnapshotSide label={labels.after} item={change.after} language={language} />
      </div>
      {change.contentChanged ? <p><Info size={12} />{labels.fullFileHash}</p> : null}
    </article>
  );
}

function SnapshotSide({
  label,
  item,
  language,
}: {
  label: string;
  item: SnapshotArtifactChange["before"];
  language: Language;
}) {
  const copy = messages[language];
  return (
    <div className={clsx("snapshot-side", !item && "empty")}>
      <span>{label}</span>
      {item ? (
        <>
          <strong>{copy.labels.resolution[item.resolution]}</strong>
          <small>{item.sourceLabel}</small>
          <code>{item.contentHash.slice(0, 12)}</code>
        </>
      ) : <strong>—</strong>}
    </div>
  );
}

function Metric({
  label,
  value,
  primary,
}: {
  label: string;
  value: number;
  primary?: boolean;
}) {
  return (
    <div className={primary ? "primary" : undefined}>
      <strong>{value}</strong><span>{label}</span>
    </div>
  );
}

function ComparePlaceholder({
  icon: Icon,
  title,
  body,
}: {
  icon: typeof Database;
  title: string;
  body?: string;
}) {
  return (
    <div className="compare-placeholder">
      <Icon size={22} />
      <strong>{title}</strong>
      {body ? <span>{body}</span> : null}
    </div>
  );
}

function comparisonCounts(comparison: ContextSnapshotComparison | null) {
  return {
    added: comparison?.changes.filter((item) => item.kind === "added").length ?? 0,
    removed: comparison?.changes.filter((item) => item.kind === "removed").length ?? 0,
    changed: comparison?.changes.filter((item) => item.kind === "changed").length ?? 0,
    content: comparison?.changes.filter((item) => item.contentChanged).length ?? 0,
    resolution: comparison?.changes.filter((item) => item.resolutionChanged).length ?? 0,
    metadata: comparison?.changes.filter((item) => item.metadataChanged).length ?? 0,
  };
}

function changeMatchesFilter(change: SnapshotArtifactChange, filter: ChangeFilter): boolean {
  if (filter === "all") return true;
  if (filter === "content") return change.contentChanged;
  if (filter === "resolution") return change.resolutionChanged;
  if (filter === "metadata") return change.metadataChanged;
  return change.kind === filter;
}

function changeFilters(
  labels: (typeof messages)[Language]["compare"],
  counts: ReturnType<typeof comparisonCounts>,
  total: number,
  complete: boolean,
): Array<{ id: ChangeFilter; label: string; count: number }> {
  return [
    { id: "all", label: labels.allChanges, count: total },
    {
      id: "added",
      label: complete ? labels.added : labels.onlyInTarget,
      count: counts.added,
    },
    {
      id: "removed",
      label: complete ? labels.removed : labels.onlyInBaseline,
      count: counts.removed,
    },
    { id: "changed", label: labels.changed, count: counts.changed },
    { id: "content", label: labels.contentFilter, count: counts.content },
    { id: "resolution", label: labels.resolutionFilter, count: counts.resolution },
    { id: "metadata", label: labels.metadataFilter, count: counts.metadata },
  ];
}

function formatDate(value: string, language: Language): string {
  return new Date(value).toLocaleString(language === "zh" ? "zh-CN" : "en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}
