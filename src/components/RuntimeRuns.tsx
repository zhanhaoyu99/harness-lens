import { useEffect, useMemo, useState } from "react";
import clsx from "clsx";
import {
  Activity,
  AlertTriangle,
  Bot,
  BrainCircuit,
  CheckCircle2,
  CircleDashed,
  Clock3,
  FileDiff,
  Image,
  Info,
  ListTree,
  MessageSquare,
  RefreshCw,
  Search,
  ShieldCheck,
  TerminalSquare,
  UserRound,
  UsersRound,
  Wrench,
} from "lucide-react";
import { messages, type Language } from "../lib/i18n";
import {
  buildTurnReplay,
  formatDuration,
  formatRuntimeStatus,
  formatRuntimeTime,
  formatStepKind,
} from "../lib/runtime";
import type {
  CodexRunDetail,
  CodexRunStep,
  CodexRuntimeSnapshot,
} from "../types";

interface RuntimeRunsProps {
  snapshot: CodexRuntimeSnapshot | null;
  detail: CodexRunDetail | null;
  selectedRunId: string | null;
  loadingSnapshot: boolean;
  loadingRun: boolean;
  error: string | null;
  language: Language;
  synthetic: boolean;
  onSelectRun: (threadId: string) => void;
  onRefresh: () => void;
}

function StepIcon({ kind }: { kind: string }) {
  const size = 15;
  switch (kind) {
    case "userMessage":
      return <UserRound size={size} />;
    case "agentMessage":
      return <Bot size={size} />;
    case "reasoning":
      return <BrainCircuit size={size} />;
    case "commandExecution":
      return <TerminalSquare size={size} />;
    case "fileChange":
      return <FileDiff size={size} />;
    case "mcpToolCall":
    case "dynamicToolCall":
      return <Wrench size={size} />;
    case "webSearch":
      return <Search size={size} />;
    case "subAgentActivity":
      return <UsersRound size={size} />;
    case "imageGeneration":
      return <Image size={size} />;
    default:
      return <Activity size={size} />;
  }
}

function RuntimePlaceholder({
  icon,
  title,
  body,
}: {
  icon: "loading" | "warning" | "empty";
  title: string;
  body?: string;
}) {
  return (
    <div className="runtime-placeholder">
      {icon === "loading" ? (
        <RefreshCw className="spin" size={22} />
      ) : icon === "warning" ? (
        <AlertTriangle size={22} />
      ) : (
        <ListTree size={22} />
      )}
      <strong>{title}</strong>
      {body ? <span>{body}</span> : null}
    </div>
  );
}

export function RuntimeRuns({
  snapshot,
  detail,
  selectedRunId,
  loadingSnapshot,
  loadingRun,
  error,
  language,
  synthetic,
  onSelectRun,
  onRefresh,
}: RuntimeRunsProps) {
  const copy = messages[language].runtime;
  const [selectedStepId, setSelectedStepId] = useState<string | null>(null);

  useEffect(() => {
    setSelectedStepId(detail?.steps[0]?.id ?? null);
  }, [detail]);

  const replay = useMemo(() => (detail ? buildTurnReplay(detail) : []), [detail]);
  const selectedStep = useMemo<CodexRunStep | null>(
    () => detail?.steps.find((step) => step.id === selectedStepId) ?? null,
    [detail, selectedStepId],
  );
  const state = snapshot?.state ?? "unavailable";
  const connected = state === "connected";
  const stateLabel = {
    connected: copy.connected,
    unavailable: copy.unavailable,
    error: copy.error,
  }[state];

  return (
    <section className="runs-page">
      <header className="runs-heading">
        <div>
          <span className="eyebrow">{copy.eyebrow}</span>
          <div className="runs-title-row">
            <h1>{copy.title}</h1>
            {synthetic ? <span className="synthetic-chip">{copy.synthetic}</span> : null}
          </div>
          <p>{copy.body}</p>
          {snapshot ? (
            <small>
              {copy.observedSummary(
                snapshot.runs.length,
                snapshot.skills.filter((skill) => skill.enabled).length,
                snapshot.hooks.filter((hook) => hook.enabled).length,
              )}
            </small>
          ) : null}
        </div>
        <div className="runtime-connection">
          <div>
            <span>{copy.connection}</span>
            <strong className={clsx(`runtime-state-${state}`)}>
              <span className="runtime-state-dot" /> {stateLabel}
            </strong>
          </div>
          <div>
            <span>{copy.version}</span>
            <strong className="mono">{snapshot?.codexVersion ?? "—"}</strong>
          </div>
          <button className="secondary-button" onClick={onRefresh} disabled={loadingSnapshot}>
            <RefreshCw size={14} className={loadingSnapshot ? "spin" : undefined} />
            {copy.refresh}
          </button>
        </div>
      </header>

      <div className="runtime-boundaries" aria-label={copy.boundariesAria}>
        <div className={clsx(detail && "active")}>
          {detail ? <CheckCircle2 size={15} /> : <CircleDashed size={15} />}
          <span><strong>{copy.boundaries.historical}</strong><small>{copy.boundariesDetail.historical}</small></span>
        </div>
        <div>
          <Info size={15} />
          <span><strong>{copy.boundaries.context}</strong><small>{copy.boundariesDetail.context}</small></span>
        </div>
        <div>
          <ShieldCheck size={15} />
          <span><strong>{copy.boundaries.outcome}</strong><small>{copy.boundariesDetail.outcome}</small></span>
        </div>
      </div>

      {loadingSnapshot && !snapshot ? (
        <RuntimePlaceholder icon="loading" title={copy.loading} />
      ) : !snapshot || !connected ? (
        <RuntimePlaceholder
          icon="warning"
          title={copy.unavailableTitle}
          body={error ?? snapshot?.message ?? copy.unavailableBody}
        />
      ) : snapshot.runs.length === 0 ? (
        <RuntimePlaceholder icon="empty" title={copy.noRunsTitle} body={copy.noRunsBody} />
      ) : (
        <div className="runtime-workbench">
          <aside className="run-list-panel">
            <div className="runtime-panel-heading">
              <span>{copy.historicalRuns}</span>
              <small>{snapshot.runs.length}</small>
            </div>
            <div className="run-list">
              {snapshot.runs.map((run) => (
                <button
                  key={run.id}
                  className={clsx(run.id === selectedRunId && "selected")}
                  aria-pressed={run.id === selectedRunId}
                  onClick={() => onSelectRun(run.id)}
                >
                  <div className="run-list-title">
                    <strong>{run.title}</strong>
                    <span className="status-pill">{formatRuntimeStatus(run.status, language)}</span>
                  </div>
                  {run.preview ? <p>{run.preview}</p> : null}
                  <div className="run-list-meta">
                    <span><Clock3 size={11} /> {formatRuntimeTime(run.updatedAt, language)}</span>
                    <span>{run.source}</span>
                    {run.gitBranch ? <span>{run.gitBranch}</span> : null}
                  </div>
                </button>
              ))}
            </div>
          </aside>

          <section className="run-replay-panel">
            {loadingRun ? (
              <RuntimePlaceholder icon="loading" title={copy.loadingRun} />
            ) : !detail ? (
              <RuntimePlaceholder icon="empty" title={copy.chooseTitle} body={error ?? copy.chooseBody} />
            ) : (
              <>
                <header className="run-detail-heading">
                  <div>
                    <span className="eyebrow">{copy.boundaries.historical}</span>
                    <h2>{detail.title}</h2>
                  </div>
                  <span className="status-pill">{formatRuntimeStatus(detail.status, language)}</span>
                  <div className="run-detail-counts">
                    <strong>{copy.turns(detail.turns.length)}</strong>
                    <span>{copy.evidenceSteps(detail.steps.length)}</span>
                  </div>
                </header>

                {detail.truncated ? (
                  <div className="replay-truncated"><AlertTriangle size={14} />{copy.truncated}</div>
                ) : null}

                <div className="linear-replay">
                  {replay.map((turn, turnIndex) => (
                    <article className="turn-card" key={turn.id}>
                      <header>
                        <span className="turn-index">{turnIndex + 1}</span>
                        <div>
                          <strong>{copy.turn(turnIndex + 1)}</strong>
                          <small>{formatRuntimeTime(turn.startedAt, language)}</small>
                        </div>
                        <span>{formatRuntimeStatus(turn.status, language)}</span>
                        <small>{copy.duration}: {formatDuration(turn.durationMs, language)}</small>
                      </header>
                      <div className="turn-steps">
                        {turn.steps.length ? turn.steps.map((step) => (
                          <button
                            key={step.id}
                            className={clsx(step.id === selectedStepId && "selected")}
                            aria-pressed={step.id === selectedStepId}
                            onClick={() => setSelectedStepId(step.id)}
                          >
                            <span className="step-connector" />
                            <span className="step-icon"><StepIcon kind={step.kind} /></span>
                            <span className="step-copy">
                              <strong>{formatStepKind(step, language)}</strong>
                              {step.detail ? <small>{step.detail}</small> : null}
                            </span>
                            {step.status ? (
                              <span className="step-status">{formatRuntimeStatus(step.status, language)}</span>
                            ) : null}
                          </button>
                        )) : <p className="turn-empty">{copy.noSteps}</p>}
                      </div>
                    </article>
                  ))}
                </div>
              </>
            )}
          </section>

          <aside className="evidence-panel">
            <div className="runtime-panel-heading">
              <span>{copy.evidenceInspector}</span>
            </div>
            {selectedStep ? (
              <div className="evidence-content">
                <div className="evidence-icon"><StepIcon kind={selectedStep.kind} /></div>
                <span className="eyebrow">{copy.kind}</span>
                <h3>{formatStepKind(selectedStep, language)}</h3>
                <dl>
                  <div><dt>{copy.status}</dt><dd>{selectedStep.status ? formatRuntimeStatus(selectedStep.status, language) : "—"}</dd></div>
                  <div><dt>{copy.turnId}</dt><dd className="mono">{selectedStep.turnId}</dd></div>
                  <div><dt>{copy.detail}</dt><dd>{selectedStep.detail ?? "—"}</dd></div>
                </dl>
                <div className="metadata-boundary">
                  <ShieldCheck size={15} />
                  <span>{copy.metadataOnly}</span>
                </div>
              </div>
            ) : (
              <div className="evidence-empty">
                <MessageSquare size={21} />
                <span>{copy.selectEvidence}</span>
              </div>
            )}
          </aside>
        </div>
      )}
    </section>
  );
}
