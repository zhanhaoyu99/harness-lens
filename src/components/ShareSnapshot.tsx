import { useState } from "react";
import { Check, Copy, LockKeyhole, Share2 } from "lucide-react";
import { messages, type Language } from "../lib/i18n";
import { buildShareSummary, shareStats } from "../lib/share";
import type { HarnessSnapshot } from "../types";

interface ShareSnapshotProps {
  snapshot: HarnessSnapshot;
  language: Language;
}

export function ShareSnapshot({ snapshot, language }: ShareSnapshotProps) {
  const [copyState, setCopyState] = useState<"idle" | "copied" | "error">("idle");
  const copy = messages[language];
  const labels = copy.shareSnapshot;
  const stats = shareStats(snapshot);

  async function copySummary() {
    try {
      await navigator.clipboard.writeText(buildShareSummary(snapshot, language));
      setCopyState("copied");
      window.setTimeout(() => setCopyState("idle"), 1800);
    } catch {
      setCopyState("error");
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
        <button className="primary-button" onClick={() => void copySummary()}>
          {copyState === "copied" ? <Check size={16} /> : <Copy size={16} />}
          {copyState === "copied"
            ? labels.copied
            : copyState === "error"
              ? labels.copyError
              : labels.copy}
        </button>
      </div>

      <article className="share-card" aria-label={labels.title}>
        <header>
          <div className="share-card-mark"><Share2 size={20} /></div>
          <div>
            <span>Harness Lens</span>
            <strong>{snapshot.workspaceName}</strong>
          </div>
          {snapshot.gitBranch ? <small>{snapshot.gitBranch}</small> : null}
        </header>

        <div className="share-metrics">
          <div><strong>{snapshot.artifacts.length}</strong><span>{labels.inventory}</span></div>
          <div><strong>{stats.resolved}</strong><span>{labels.resolved}</span></div>
          <div><strong>{stats.driftedItems}</strong><span>{labels.drift}</span></div>
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
    </section>
  );
}
