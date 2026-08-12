import { useEffect, useMemo, useRef, useState } from "react";
import clsx from "clsx";
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  ChevronRight,
  CircleDashed,
  FileSearch,
  FolderOpen,
  GitCompareArrows,
  Info,
  Languages,
  LayoutDashboard,
  List,
  Network,
  RefreshCw,
  Search,
  Share2,
  Sparkles,
} from "lucide-react";
import { HarnessMap, type MapFilter } from "./components/HarnessMap";
import { HarnessTable } from "./components/HarnessTable";
import { Inspector } from "./components/Inspector";
import { RuntimeRuns } from "./components/RuntimeRuns";
import { ShareSnapshot } from "./components/ShareSnapshot";
import { driftCount, effectiveCount, filterArtifacts } from "./lib/artifacts";
import {
  getInitialLanguage,
  localizeWarning,
  messages,
  persistLanguage,
  type Language,
} from "./lib/i18n";
import {
  sampleRunDetail,
  sampleRuntimeSnapshot,
  sampleSnapshot,
} from "./lib/sample";
import {
  chooseWorkspace,
  inspectRuntime,
  isTauriRuntime,
  loadDefaultWorkspace,
  loadRuntimeRun,
  rescanWorkspace,
  revealSource,
} from "./lib/tauri";
import type {
  CodexRunDetail,
  CodexRuntimeSnapshot,
  ExplorerMode,
  HarnessArtifact,
  HarnessKind,
  HarnessSnapshot,
  PrimarySection,
} from "./types";

const navItems: Array<{
  id: PrimarySection;
  icon: typeof LayoutDashboard;
  planned?: boolean;
}> = [
  { id: "overview", icon: LayoutDashboard },
  { id: "items", icon: FileSearch },
  { id: "runs", icon: Activity },
  { id: "compare", icon: GitCompareArrows, planned: true },
  { id: "share", icon: Share2 },
];

function StageCard({
  label,
  value,
  detail,
  active,
}: {
  label: string;
  value: string;
  detail: string;
  active?: boolean;
}) {
  return (
    <div className={clsx("stage-card", active && "active")}>
      <div className="stage-icon">
        {active ? <CheckCircle2 size={16} /> : <CircleDashed size={16} />}
      </div>
      <div>
        <span>{label}</span>
        <strong>{value}</strong>
        <small>{detail}</small>
      </div>
    </div>
  );
}

function EmptyWorkspace({
  language,
  onChoose,
}: {
  language: Language;
  onChoose: () => void;
}) {
  const copy = messages[language].emptyWorkspace;
  return (
    <div className="welcome-card">
      <div className="welcome-visual">
        <div className="orbit orbit-one" />
        <div className="orbit orbit-two" />
        <Network size={42} />
      </div>
      <span className="eyebrow">{copy.eyebrow}</span>
      <h1>{copy.title}</h1>
      <p>{copy.body}</p>
      <button className="primary-button" onClick={onChoose}>
        <FolderOpen size={17} />
        {copy.choose}
      </button>
      <div className="privacy-note">
        <CheckCircle2 size={15} /> {copy.privacy}
      </div>
    </div>
  );
}

function FutureSection({
  section,
  language,
}: {
  section: "compare";
  language: Language;
}) {
  const copy = messages[language].future;
  const content = {
    compare: {
      ...copy.compare,
      icon: GitCompareArrows,
    },
  }[section];
  const Icon = content.icon;
  return (
    <div className="future-section">
      <div className="future-icon"><Icon size={28} /></div>
      <span className="eyebrow">{copy.eyebrow}</span>
      <h2>{content.title}</h2>
      <p>{content.body}</p>
      <div className="future-contract">
        <Sparkles size={16} /> {copy.contract}
      </div>
    </div>
  );
}

export default function App() {
  const tauri = isTauriRuntime();
  const initialSampleRunId = sampleRuntimeSnapshot.runs[0]?.id ?? null;
  const [language, setLanguage] = useState<Language>(getInitialLanguage);
  const [snapshot, setSnapshot] = useState<HarnessSnapshot | null>(tauri ? null : sampleSnapshot);
  const [runtimeSnapshot, setRuntimeSnapshot] = useState<CodexRuntimeSnapshot | null>(
    tauri ? null : sampleRuntimeSnapshot,
  );
  const [selectedRunId, setSelectedRunId] = useState<string | null>(
    tauri ? null : initialSampleRunId,
  );
  const [runDetail, setRunDetail] = useState<CodexRunDetail | null>(
    tauri || !initialSampleRunId ? null : sampleRunDetail(initialSampleRunId),
  );
  const [section, setSection] = useState<PrimarySection>("overview");
  const [mode, setMode] = useState<ExplorerMode>("list");
  const [search, setSearch] = useState("");
  const [mapFilter, setMapFilter] = useState<MapFilter>({});
  const [selectedArtifactId, setSelectedArtifactId] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [runtimeLoading, setRuntimeLoading] = useState(false);
  const [runLoading, setRunLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const runLoadSequence = useRef(0);
  const copy = messages[language];

  function changeLanguage(nextLanguage: Language) {
    setLanguage(nextLanguage);
    persistLanguage(nextLanguage);
  }

  async function performScan(loader: () => Promise<HarnessSnapshot | null>) {
    setScanning(true);
    setError(null);
    try {
      const result = await loader();
      if (!result) return;
      runLoadSequence.current += 1;
      setRuntimeSnapshot(null);
      setSelectedRunId(null);
      setRunDetail(null);
      setRuntimeError(null);
      setSnapshot(result);
      setSelectedArtifactId(null);
      setMapFilter({});
      await performRuntimeScan();
    } catch (scanError) {
      setError(scanError instanceof Error ? scanError.message : String(scanError));
    } finally {
      setScanning(false);
    }
  }

  async function performRuntimeScan() {
    setRuntimeLoading(true);
    setRuntimeError(null);
    setSelectedRunId(null);
    setRunDetail(null);
    runLoadSequence.current += 1;
    try {
      const result = await inspectRuntime();
      setRuntimeSnapshot(result);
      if (result.state !== "connected" && result.message) {
        setRuntimeError(result.message);
      }
    } catch (runtimeScanError) {
      const message = runtimeScanError instanceof Error
        ? runtimeScanError.message
        : String(runtimeScanError);
      setRuntimeError(message);
      setRuntimeSnapshot({
        state: "error",
        codexVersion: null,
        observedAt: new Date().toISOString(),
        message,
        skills: [],
        hooks: [],
        runs: [],
      });
    } finally {
      setRuntimeLoading(false);
    }
  }

  async function handleSelectRun(threadId: string) {
    setSelectedRunId(threadId);
    setRunDetail(null);
    setRuntimeError(null);
    if (!tauri) {
      setRunDetail(sampleRunDetail(threadId));
      return;
    }

    const sequence = ++runLoadSequence.current;
    setRunLoading(true);
    try {
      const result = await loadRuntimeRun(threadId);
      if (sequence === runLoadSequence.current) setRunDetail(result);
    } catch (runError) {
      if (sequence === runLoadSequence.current) {
        setRuntimeError(runError instanceof Error ? runError.message : String(runError));
      }
    } finally {
      if (sequence === runLoadSequence.current) setRunLoading(false);
    }
  }

  function handleRefreshRuntime() {
    if (tauri && snapshot) {
      void performRuntimeScan();
      return;
    }
    setRuntimeSnapshot(sampleRuntimeSnapshot);
    setSelectedRunId(initialSampleRunId);
    setRunDetail(initialSampleRunId ? sampleRunDetail(initialSampleRunId) : null);
    setRuntimeError(null);
  }

  function handleRescan() {
    if (tauri && snapshot) {
      void performScan(rescanWorkspace);
      return;
    }
    setSnapshot(sampleSnapshot);
    setSelectedArtifactId(null);
    setMapFilter({});
    handleRefreshRuntime();
  }

  useEffect(() => {
    document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
  }, [language]);

  useEffect(() => {
    if (!tauri) return;
    let cancelled = false;
    async function restoreWorkspace() {
      const initialSnapshot = await loadDefaultWorkspace();
      if (initialSnapshot && !cancelled) {
        await performScan(async () => initialSnapshot);
      }
    }
    void restoreWorkspace();
    return () => {
      cancelled = true;
    };
  }, [tauri]);

  const filteredArtifacts = useMemo(
    () =>
      snapshot
        ? filterArtifacts(snapshot.artifacts, { ...mapFilter, search })
        : [],
    [snapshot, mapFilter, search],
  );

  const selectedArtifact = useMemo<HarnessArtifact | null>(
    () => snapshot?.artifacts.find((item) => item.id === selectedArtifactId) ?? null,
    [snapshot, selectedArtifactId],
  );

  const groupedArtifacts = selectedArtifact ? [] : filteredArtifacts;
  const visibleWarnings = snapshot?.warnings.filter(
    (warning) => !(
      runtimeSnapshot?.state === "connected" &&
      (warning.id === "runtime-not-connected" || warning.id === "observed-not-connected")
    ),
  ) ?? [];

  async function handleChooseWorkspace() {
    if (!tauri) return;
    await performScan(() => chooseWorkspace(copy.workspace.chooseDialogTitle));
  }

  function selectKind(kind: HarnessKind) {
    setSection("items");
    setMode("list");
    setMapFilter({ kind });
    setSelectedArtifactId(null);
  }

  const kinds = snapshot
    ? Array.from(new Set(snapshot.artifacts.map((artifact) => artifact.kind))).sort()
    : [];

  return (
    <div className={clsx("app-frame", section !== "overview" && section !== "items" && "focused-view")}>
      <aside className="sidebar">
        <div className="brand-lockup">
          <div className="brand-mark"><Network size={19} /></div>
          <div><strong>Harness Lens</strong><span>{copy.brandSubtitle}</span></div>
        </div>

        <nav className="primary-nav" aria-label={copy.common.primaryNavigation}>
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                className={clsx(section === item.id && "active")}
                aria-current={section === item.id ? "page" : undefined}
                onClick={() => setSection(item.id)}
              >
                <Icon size={16} />
                <span>{copy.nav[item.id]}</span>
                {item.planned ? <small>{copy.common.next}</small> : null}
              </button>
            );
          })}
        </nav>

        {snapshot ? (
          <div className="sidebar-group">
            <div className="sidebar-group-title"><span>{copy.common.inventory}</span><small>{snapshot.artifacts.length}</small></div>
            {kinds.map((kind) => {
              const count = snapshot.artifacts.filter((artifact) => artifact.kind === kind).length;
              return (
                <button key={kind} onClick={() => selectKind(kind)}>
                  <span>{copy.labels.kind[kind]}</span><small>{count}</small>
                </button>
              );
            })}
          </div>
        ) : null}

        <div className="sidebar-footer">
          <div className="local-chip"><span className="local-dot" />{copy.common.localOnly}</div>
          <span>{copy.common.readOnlyVersion}</span>
        </div>
      </aside>

      <main className="main-column">
        <header className="topbar">
          <div className="workspace-context">
            <span className="eyebrow">{copy.workspace.label}</span>
            <div>
              <strong>{snapshot?.workspaceName ?? copy.workspace.noneSelected}</strong>
              {snapshot?.gitBranch ? <span className="branch-chip">{snapshot.gitBranch}</span> : null}
            </div>
            {snapshot ? <small>{snapshot.workspacePath}</small> : null}
          </div>
          <div className="topbar-actions">
            <div className="language-setting" role="group" aria-label={copy.languageSetting} title={copy.languageSetting}>
              <Languages size={14} />
              <button
                className={clsx(language === "zh" && "active")}
                aria-pressed={language === "zh"}
                onClick={() => changeLanguage("zh")}
              >
                中文
              </button>
              <span>/</span>
              <button
                className={clsx(language === "en" && "active")}
                aria-pressed={language === "en"}
                onClick={() => changeLanguage("en")}
              >
                English
              </button>
            </div>
            {snapshot ? (
              <button
                className="secondary-button"
                onClick={handleRescan}
                disabled={scanning}
              >
                <RefreshCw size={15} className={scanning ? "spin" : undefined} />
                {copy.workspace.rescan}
              </button>
            ) : null}
            <button className="secondary-button" onClick={() => void handleChooseWorkspace()} disabled={!tauri}>
              <FolderOpen size={15} /> {copy.workspace.open}
            </button>
          </div>
        </header>

        {error ? <div className="error-banner"><AlertTriangle size={16} />{error}</div> : null}

        <div className="main-scroll">
          {!snapshot ? (
            <EmptyWorkspace language={language} onChoose={() => void handleChooseWorkspace()} />
          ) : section === "share" ? (
            <ShareSnapshot snapshot={snapshot} language={language} />
          ) : section === "runs" ? (
            <RuntimeRuns
              snapshot={runtimeSnapshot}
              detail={runDetail}
              selectedRunId={selectedRunId}
              loadingSnapshot={runtimeLoading}
              loadingRun={runLoading}
              error={runtimeError}
              language={language}
              synthetic={!tauri}
              onSelectRun={(threadId) => void handleSelectRun(threadId)}
              onRefresh={handleRefreshRuntime}
            />
          ) : section === "compare" ? (
            <FutureSection section={section} language={language} />
          ) : (
            <>
              <section className="stage-strip" aria-label={copy.stages.ariaLabel}>
                <StageCard label={copy.stages.defined} value={String(snapshot.artifacts.length)} detail={copy.stages.definedDetail} active />
                <ChevronRight size={16} />
                <StageCard label={copy.stages.resolved} value={String(effectiveCount(snapshot.artifacts))} detail={copy.stages.resolvedDetail} active />
                <ChevronRight size={16} />
                <StageCard
                  label={copy.stages.observed}
                  value={runtimeSnapshot?.state === "connected" ? String(runtimeSnapshot.runs.length) : "—"}
                  detail={copy.stages.observedDetail}
                  active={runtimeSnapshot?.state === "connected"}
                />
                <ChevronRight size={16} />
                <StageCard label={copy.stages.evaluated} value="—" detail={copy.stages.evaluatedDetail} />
              </section>

              <section className="overview-heading">
                <div>
                  <span className="eyebrow">{section === "overview" ? copy.overview.currentHarness : copy.overview.inventory}</span>
                  <h1>{section === "overview" ? copy.overview.title : copy.overview.itemsTitle}</h1>
                  <p>{copy.overview.summary(
                    snapshot.artifacts.length,
                    new Set(snapshot.artifacts.map((item) => item.provider)).size,
                    driftCount(snapshot.artifacts),
                  )}</p>
                </div>
                <div className="scan-meta">
                  <span>{copy.overview.snapshot}</span>
                  <strong>{new Date(snapshot.scannedAt).toLocaleTimeString(language === "zh" ? "zh-CN" : "en-US", { hour: "2-digit", minute: "2-digit" })}</strong>
                </div>
              </section>

              {visibleWarnings.length ? (
                <section className="warning-row">
                  {visibleWarnings.slice(0, 3).map((warning) => {
                    const localizedWarning = localizeWarning(warning, language);
                    return (
                      <button key={warning.id} onClick={() => {
                        const id = warning.artifactIds[0];
                        if (id) setSelectedArtifactId(id);
                      }}>
                        {warning.severity === "warning" ? <AlertTriangle size={16} /> : <Info size={16} />}
                        <span><strong>{localizedWarning.title}</strong><small>{localizedWarning.detail}</small></span>
                      </button>
                    );
                  })}
                </section>
              ) : null}

              <section className="explorer-card">
                <div className="explorer-toolbar">
                  <div className="segmented-control">
                    <button
                      className={clsx(mode === "map" && "active")}
                      aria-pressed={mode === "map"}
                      onClick={() => setMode("map")}
                    >
                      <Network size={15} /> {copy.explorer.map}
                    </button>
                    <button
                      className={clsx(mode === "list" && "active")}
                      aria-pressed={mode === "list"}
                      onClick={() => setMode("list")}
                    >
                      <List size={15} /> {copy.explorer.list}
                    </button>
                  </div>
                  <div className="toolbar-spacer" />
                  {(mapFilter.provider || mapFilter.kind) ? (
                    <button className="filter-chip" onClick={() => setMapFilter({})}>
                      {mapFilter.provider ? copy.labels.provider[mapFilter.provider] : ""}
                      {mapFilter.provider && mapFilter.kind ? " · " : ""}
                      {mapFilter.kind ? copy.labels.kind[mapFilter.kind] : ""}
                      <span>×</span>
                    </button>
                  ) : null}
                  <label className="search-field">
                    <Search size={15} />
                    <input
                      value={search}
                      aria-label={copy.explorer.search}
                      onChange={(event) => setSearch(event.target.value)}
                      placeholder={copy.explorer.search}
                    />
                  </label>
                </div>

                {mode === "map" && !search && !mapFilter.provider && !mapFilter.kind ? (
                  <HarnessMap
                    snapshot={snapshot}
                    language={language}
                    onFilter={(filter) => {
                      setMapFilter(filter);
                      setSelectedArtifactId(null);
                    }}
                  />
                ) : (
                  <HarnessTable
                    artifacts={filteredArtifacts}
                    language={language}
                    workspacePath={snapshot.workspacePath}
                    selectedId={selectedArtifactId}
                    onSelect={setSelectedArtifactId}
                  />
                )}
              </section>
            </>
          )}
        </div>
      </main>

      {section === "overview" || section === "items" ? (
        <Inspector
          artifact={selectedArtifact}
          language={language}
          workspacePath={snapshot?.workspacePath ?? null}
          groupedArtifacts={groupedArtifacts.length === snapshot?.artifacts.length ? [] : groupedArtifacts}
          onSelect={setSelectedArtifactId}
          onOpenSource={(path) => {
            if (tauri) void revealSource(path);
          }}
        />
      ) : null}
    </div>
  );
}
