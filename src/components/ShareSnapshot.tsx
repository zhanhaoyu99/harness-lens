import { useEffect, useRef, useState } from "react";
import {
  AlertCircle,
  Check,
  Copy,
  LockKeyhole,
  RefreshCw,
  Share2,
  ShieldCheck,
} from "lucide-react";
import { messages, type Language } from "../lib/i18n";
import { buildSyntheticCompatibilityExample, shareStats } from "../lib/share";
import { generateCompatibilityReport } from "../lib/tauri";
import type { CompatibilityReportOutput, HarnessSnapshot } from "../types";

interface ShareSnapshotProps {
  snapshot: HarnessSnapshot;
  language: Language;
  synthetic: boolean;
  hasUnsavedMemory: boolean;
}

type CopyState = "idle" | "copying" | "copied" | "error";

export function ShareSnapshot({
  snapshot,
  language,
  synthetic,
  hasUnsavedMemory,
}: ShareSnapshotProps) {
  const [reportOutput, setReportOutput] = useState<CompatibilityReportOutput | null>(null);
  const [generating, setGenerating] = useState(false);
  const [generationError, setGenerationError] = useState<string | null>(null);
  const [copyState, setCopyState] = useState<CopyState>("idle");
  const generationSequence = useRef(0);
  const copySequence = useRef(0);
  const generatingRef = useRef(false);
  const copyingRef = useRef(false);
  const mountedRef = useRef(true);
  const snapshotKey = `${snapshot.workspacePath}\u0000${snapshot.scannedAt}`;
  const snapshotKeyRef = useRef(snapshotKey);
  snapshotKeyRef.current = snapshotKey;

  const copy = messages[language];
  const labels = copy.shareSnapshot;
  const stats = shareStats(snapshot);
  const syntheticMarkdown = synthetic
    ? buildSyntheticCompatibilityExample(snapshot)
    : null;

  useEffect(() => {
    // React Strict Mode intentionally replays effect setup and cleanup in
    // development. Restore the mounted flag on every setup so source builds do
    // not discard a valid report after that replay.
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      generationSequence.current += 1;
      copySequence.current += 1;
    };
  }, []);

  useEffect(() => {
    generationSequence.current += 1;
    copySequence.current += 1;
    generatingRef.current = false;
    copyingRef.current = false;
    setGenerating(false);
    setReportOutput(null);
    setGenerationError(null);
    setCopyState("idle");
  }, [snapshotKey]);

  async function generateReport() {
    if (synthetic || generatingRef.current) return;
    // A newly generated report invalidates any clipboard operation that was
    // copying the previous report. Never show a stale "copied" status beside
    // newer Markdown.
    copySequence.current += 1;
    copyingRef.current = false;
    const sourceSnapshotKey = snapshotKeyRef.current;
    const operation = ++generationSequence.current;
    const operationIsCurrent = () => (
      mountedRef.current
      && operation === generationSequence.current
      && sourceSnapshotKey === snapshotKeyRef.current
    );

    generatingRef.current = true;
    setGenerating(true);
    setReportOutput(null);
    setGenerationError(null);
    setCopyState("idle");
    try {
      const output = await generateCompatibilityReport();
      if (!operationIsCurrent()) return;
      setReportOutput(output);
    } catch (error) {
      if (!operationIsCurrent()) return;
      setGenerationError(error instanceof Error ? error.message : String(error));
    } finally {
      if (operationIsCurrent()) {
        generatingRef.current = false;
        setGenerating(false);
      }
    }
  }

  async function copyReport() {
    if (synthetic || !reportOutput || copyingRef.current) return;
    const sourceSnapshotKey = snapshotKeyRef.current;
    const operation = ++copySequence.current;
    const operationIsCurrent = () => (
      mountedRef.current
      && operation === copySequence.current
      && sourceSnapshotKey === snapshotKeyRef.current
    );
    copyingRef.current = true;
    setCopyState("copying");
    try {
      await navigator.clipboard.writeText(reportOutput.markdown);
      if (operationIsCurrent()) setCopyState("copied");
    } catch {
      if (operationIsCurrent()) setCopyState("error");
    } finally {
      if (operationIsCurrent()) copyingRef.current = false;
    }
  }

  return (
    <section className="share-page">
      <div className="share-page-heading">
        <div>
          <span className="eyebrow">{labels.eyebrow}</span>
          <h1>{labels.title}</h1>
          <p>{labels.body}</p>
        </div>
        {!synthetic ? (
          <button
            className="primary-button"
            disabled={generating}
            onClick={() => void generateReport()}
          >
            <RefreshCw size={16} className={generating ? "spin" : undefined} />
            {generating ? labels.generating : labels.generate}
          </button>
        ) : null}
      </div>

      <article className="share-card" aria-label={labels.currentPreviewTitle}>
        <header>
          <div className="share-card-mark"><Share2 size={20} /></div>
          <div>
            <span>{labels.currentPreview}</span>
            <strong>{snapshot.workspaceName}</strong>
          </div>
          {snapshot.gitBranch ? <small>{snapshot.gitBranch}</small> : null}
        </header>

        <div className="share-metrics">
          <div><strong>{snapshot.artifacts.length}</strong><span>{labels.inventory}</span></div>
          <div><strong>{stats.resolved}</strong><span>{labels.resolved}</span></div>
          <div><strong>{stats.differenceGroups}</strong><span>{labels.drift}</span></div>
          <div><strong>{stats.duplicateGroups}</strong><span>{labels.duplicates}</span></div>
          <div><strong>{stats.unknown}</strong><span>{labels.unknown}</span></div>
        </div>

        <div className="share-kind-grid">
          {stats.byKind.map(({ kind, count }) => (
            <div key={kind}>
              <span>{copy.labels.kind[kind]}</span>
              <strong>{count}</strong>
            </div>
          ))}
        </div>

        <footer>
          <LockKeyhole size={14} />
          <span>{labels.privacy}</span>
          <time>{new Date(snapshot.scannedAt).toLocaleString(language === "zh" ? "zh-CN" : "en-US")}</time>
        </footer>
      </article>

      <div className={`report-disk-boundary${hasUnsavedMemory ? " dirty" : ""}`}>
        <ShieldCheck size={16} />
        <span>{hasUnsavedMemory ? labels.unsavedDiskBoundary : labels.diskBoundary}</span>
      </div>

      <section className={`compatibility-report-panel${synthetic ? " synthetic" : ""}`}>
        <header>
          <div>
            <span className="eyebrow">{labels.reportEyebrow}</span>
            <h2>{synthetic ? labels.syntheticTitle : labels.previewTitle}</h2>
          </div>
          {synthetic ? (
            <span className="synthetic-report-badge">{labels.syntheticBadge}</span>
          ) : reportOutput ? (
            <span className="report-schema-badge">
              {labels.schemaVersion(reportOutput.report.reportSchemaVersion)}
            </span>
          ) : null}
        </header>

        {synthetic && syntheticMarkdown ? (
          <>
            <div className="synthetic-report-warning" role="note">
              <AlertCircle size={16} />
              <span>{labels.syntheticBoundary}</span>
            </div>
            <pre className="compatibility-markdown-preview" aria-label={labels.syntheticPreviewLabel}>
              {syntheticMarkdown}
            </pre>
          </>
        ) : generationError ? (
          <div className="report-error" role="alert">
            <AlertCircle size={16} />
            <div>
              <strong>{labels.generateErrorTitle}</strong>
              <span>{generationError}</span>
            </div>
          </div>
        ) : reportOutput ? (
          <>
            <p className="report-review-instruction">
              {labels.reviewInstruction(new Date(reportOutput.scannedAt).toLocaleString(
                language === "zh" ? "zh-CN" : "en-US",
              ))}
            </p>
            <pre className="compatibility-markdown-preview" aria-label={labels.previewLabel}>
              {reportOutput.markdown}
            </pre>
            <div className="report-copy-row">
              <p aria-live="polite" role="status">
                {copyState === "copied"
                  ? labels.copiedDetail
                  : copyState === "error"
                    ? labels.copyErrorDetail
                    : labels.copyBoundary}
              </p>
              <button
                className="secondary-button"
                disabled={copyState === "copying"}
                onClick={() => void copyReport()}
              >
                {copyState === "copied" ? <Check size={16} /> : <Copy size={16} />}
                {copyState === "copying"
                  ? labels.copying
                  : copyState === "copied"
                    ? labels.copied
                    : copyState === "error"
                      ? labels.retryCopy
                      : labels.copyReport}
              </button>
            </div>
          </>
        ) : (
          <div className="report-empty-state" aria-live="polite">
            {generating ? (
              <>
                <RefreshCw className="spin" size={22} />
                <strong>{labels.generating}</strong>
                <span>{labels.generatingDetail}</span>
              </>
            ) : (
              <>
                <ShieldCheck size={22} />
                <strong>{labels.emptyTitle}</strong>
                <span>{labels.emptyBody}</span>
              </>
            )}
          </div>
        )}
      </section>
    </section>
  );
}
