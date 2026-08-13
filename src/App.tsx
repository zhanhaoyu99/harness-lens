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
} from "lucide-react";
import { HarnessMap, type MapFilter } from "./components/HarnessMap";
import { HarnessTable } from "./components/HarnessTable";
import { Inspector, type LoadedMemoryState } from "./components/Inspector";
import { RuntimeRuns } from "./components/RuntimeRuns";
import { ShareSnapshot } from "./components/ShareSnapshot";
import { SnapshotCompare } from "./components/SnapshotCompare";
import {
  counterpartDifferenceCount,
  effectiveCount,
  filterArtifacts,
  providerFacetCounts,
} from "./lib/artifacts";
import {
  getInitialLanguage,
  localizeWarning,
  messages,
  persistLanguage,
  type Language,
} from "./lib/i18n";
import {
  sampleContextSnapshotHistory,
  sampleRunDetail,
  sampleRuntimeSnapshot,
  sampleSnapshot,
  sampleStoredContextSnapshots,
} from "./lib/sample";
import { compareStoredSnapshots, safeStoredSnapshot } from "./lib/snapshots";
import {
  captureContextSnapshot,
  chooseWorkspace,
  clearContextSnapshotHistory,
  compareContextSnapshots,
  inspectRuntime,
  isTauriRuntime,
  listContextSnapshots,
  loadContextSnapshot,
  loadDefaultWorkspace,
  loadMemoryArtifact,
  loadRuntimeRun,
  rescanWorkspace,
  revealSource,
  saveMemoryArtifact,
} from "./lib/tauri";
import type {
  CodexRunDetail,
  CodexRuntimeSnapshot,
  ContextSnapshotComparison,
  ContextSnapshotSummary,
  ExplorerMode,
  HarnessArtifact,
  HarnessKind,
  HarnessProvider,
  HarnessScope,
  HarnessSnapshot,
  MemorySaveError,
  PrimarySection,
  StoredContextSnapshot,
} from "./types";

const navItems: Array<{
  id: PrimarySection;
  icon: typeof LayoutDashboard;
  planned?: boolean;
}> = [
  { id: "overview", icon: LayoutDashboard },
  { id: "items", icon: FileSearch },
  { id: "runs", icon: Activity },
  { id: "compare", icon: GitCompareArrows },
  { id: "share", icon: Share2 },
];

const providerDisplayOrder: readonly HarnessProvider[] = ["codex", "claude", "shared", "plugin"];

function filtersMatch(left: MapFilter, right: MapFilter): boolean {
  return left.provider === right.provider
    && left.kind === right.kind
    && left.scope === right.scope;
}

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
  const [loadedMemory, setLoadedMemory] = useState<LoadedMemoryState | null>(null);
  const [memoryLoading, setMemoryLoading] = useState(false);
  const [memorySaving, setMemorySaving] = useState(false);
  const [memoryError, setMemoryError] = useState<string | null>(null);
  const [memoryFeedback, setMemoryFeedback] = useState<"saved" | "savedRefreshFailed" | "cancelled" | "discarded" | null>(null);
  const [snapshotHistory, setSnapshotHistory] = useState<ContextSnapshotSummary[]>(
    tauri ? [] : sampleContextSnapshotHistory,
  );
  const [baseCaptureId, setBaseCaptureId] = useState(
    tauri ? "" : (sampleContextSnapshotHistory[1]?.captureId ?? ""),
  );
  const [targetCaptureId, setTargetCaptureId] = useState(
    tauri ? "" : (sampleContextSnapshotHistory[0]?.captureId ?? ""),
  );
  const [snapshotComparison, setSnapshotComparison] = useState<ContextSnapshotComparison | null>(
    tauri
      ? null
      : compareStoredSnapshots(sampleStoredContextSnapshots[1], sampleStoredContextSnapshots[0]),
  );
  const [inspectedContextSnapshot, setInspectedContextSnapshot] = useState<StoredContextSnapshot | null>(
    tauri ? null : sampleStoredContextSnapshots[0],
  );
  const [snapshotHistoryLoading, setSnapshotHistoryLoading] = useState(false);
  const [snapshotCapturing, setSnapshotCapturing] = useState(false);
  const [snapshotComparing, setSnapshotComparing] = useState(false);
  const [contextSnapshotLoading, setContextSnapshotLoading] = useState(false);
  const [snapshotHistoryClearing, setSnapshotHistoryClearing] = useState(false);
  const [snapshotHistoryFeedback, setSnapshotHistoryFeedback] = useState<"captured" | "cleared" | null>(null);
  const [snapshotHistoryWarning, setSnapshotHistoryWarning] = useState<string | null>(null);
  const [snapshotHistoryError, setSnapshotHistoryError] = useState<string | null>(null);
  const activeMemorySaveSequence = useRef<number | null>(null);
  const runLoadSequence = useRef(0);
  const memoryLoadSequence = useRef(0);
  const memoryMutationSequence = useRef(0);
  const scanSequence = useRef(0);
  const runtimeScanSequence = useRef(0);
  const activeRuntimeWorkspacePath = useRef<string | null>(
    tauri ? null : sampleSnapshot.workspacePath,
  );
  const snapshotHistorySequence = useRef(0);
  const contextSnapshotLoadSequence = useRef(0);
  const snapshotCaptureSequence = useRef(0);
  const snapshotComparisonSequence = useRef(0);
  const syntheticSnapshots = useRef<StoredContextSnapshot[]>(sampleStoredContextSnapshots);
  const copy = messages[language];

  function clearLoadedMemory() {
    memoryLoadSequence.current += 1;
    activeMemorySaveSequence.current = null;
    setLoadedMemory(null);
    setMemoryLoading(false);
    setMemorySaving(false);
    setMemoryError(null);
    setMemoryFeedback(null);
  }

  function resetRuntimeForWorkspace(workspacePath: string) {
    activeRuntimeWorkspacePath.current = workspacePath;
    runtimeScanSequence.current += 1;
    runLoadSequence.current += 1;
    setRuntimeSnapshot(null);
    setRuntimeLoading(false);
    setSelectedRunId(null);
    setRunDetail(null);
    setRunLoading(false);
    setRuntimeError(null);
  }

  function suspendRuntimeForWorkspaceScan(): number {
    activeRuntimeWorkspacePath.current = null;
    runtimeScanSequence.current += 1;
    runLoadSequence.current += 1;
    setRuntimeLoading(true);
    setRunLoading(false);
    setRuntimeError(null);
    return runtimeScanSequence.current;
  }

  function resetSnapshotHistoryUi() {
    snapshotHistorySequence.current += 1;
    contextSnapshotLoadSequence.current += 1;
    snapshotCaptureSequence.current += 1;
    snapshotComparisonSequence.current += 1;
    setSnapshotHistory([]);
    setBaseCaptureId("");
    setTargetCaptureId("");
    setSnapshotComparison(null);
    setInspectedContextSnapshot(null);
    setSnapshotHistoryLoading(false);
    setContextSnapshotLoading(false);
    setSnapshotCapturing(false);
    setSnapshotComparing(false);
    setSnapshotHistoryClearing(false);
    setSnapshotHistoryFeedback(null);
    setSnapshotHistoryWarning(null);
    setSnapshotHistoryError(null);
  }

  function applySnapshotHistory(
    nextHistory: ContextSnapshotSummary[],
    preferredTargetId?: string,
    preferredBaseId?: string,
  ) {
    contextSnapshotLoadSequence.current += 1;
    snapshotComparisonSequence.current += 1;
    const sorted = [...nextHistory].sort(
      (left, right) => right.capturedAt.localeCompare(left.capturedAt),
    );
    const hasCapture = (captureId: string | undefined) =>
      Boolean(captureId && sorted.some((item) => item.captureId === captureId));
    const nextTarget = hasCapture(preferredTargetId)
      ? preferredTargetId!
      : hasCapture(targetCaptureId)
        ? targetCaptureId
        : (sorted[0]?.captureId ?? "");
    const nextBase = hasCapture(preferredBaseId) && preferredBaseId !== nextTarget
      ? preferredBaseId!
      : hasCapture(baseCaptureId) && baseCaptureId !== nextTarget
        ? baseCaptureId
        : (sorted.find((item) => item.captureId !== nextTarget)?.captureId ?? "");

    setSnapshotHistory(sorted);
    setBaseCaptureId(nextBase);
    setTargetCaptureId(nextTarget);
    setSnapshotComparison(null);
    setInspectedContextSnapshot(null);
    setSnapshotHistoryLoading(false);
    setContextSnapshotLoading(false);
    setSnapshotComparing(false);
  }

  function confirmUnsavedMemoryLoss(): boolean {
    if (!loadedMemory || loadedMemory.draft === loadedMemory.document.content) return true;
    return window.confirm(copy.inspector.discardUnsavedMemoryConfirm);
  }

  function changeLanguage(nextLanguage: Language) {
    setLanguage(nextLanguage);
    persistLanguage(nextLanguage);
  }

  async function performScan(
    loader: () => Promise<HarnessSnapshot | null>,
    unsavedConfirmed = false,
  ) {
    if (!unsavedConfirmed && !confirmUnsavedMemoryLoss()) return;
    memoryMutationSequence.current += 1;
    const operation = ++scanSequence.current;
    const previousWorkspacePath = snapshot?.workspacePath ?? activeRuntimeWorkspacePath.current;
    suspendRuntimeForWorkspaceScan();
    clearLoadedMemory();
    setScanning(true);
    setError(null);
    try {
      const result = await loader();
      if (operation !== scanSequence.current) return;
      if (!result) {
        activeRuntimeWorkspacePath.current = previousWorkspacePath;
        setRuntimeLoading(false);
        return;
      }
      resetRuntimeForWorkspace(result.workspacePath);
      if (snapshot?.workspacePath !== result.workspacePath) {
        resetSnapshotHistoryUi();
      }
      setSnapshot(result);
      setSelectedArtifactId(null);
      setMapFilter({});
      await performRuntimeScan(result.workspacePath);
    } catch (scanError) {
      if (operation === scanSequence.current) {
        activeRuntimeWorkspacePath.current = previousWorkspacePath;
        setRuntimeLoading(false);
        setError(scanError instanceof Error ? scanError.message : String(scanError));
      }
    } finally {
      if (operation === scanSequence.current) setScanning(false);
    }
  }

  async function performRuntimeScan(workspacePath: string) {
    if (activeRuntimeWorkspacePath.current !== workspacePath) return;
    const operation = ++runtimeScanSequence.current;
    const operationIsCurrent = () => (
      operation === runtimeScanSequence.current
      && workspacePath === activeRuntimeWorkspacePath.current
    );
    setRuntimeLoading(true);
    setRuntimeError(null);
    setSelectedRunId(null);
    setRunDetail(null);
    runLoadSequence.current += 1;
    try {
      const result = await inspectRuntime();
      if (!operationIsCurrent()) return;
      setRuntimeSnapshot(result);
      if (result.state !== "connected" && result.message) {
        setRuntimeError(result.message);
      }
    } catch (runtimeScanError) {
      if (!operationIsCurrent()) return;
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
      if (operationIsCurrent()) setRuntimeLoading(false);
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
      void performRuntimeScan(snapshot.workspacePath);
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
    if (!confirmUnsavedMemoryLoss()) return;
    setSnapshot(sampleSnapshot);
    setSelectedArtifactId(null);
    setMapFilter({});
    clearLoadedMemory();
    handleRefreshRuntime();
  }

  function handleSelectArtifact(id: string) {
    const target = snapshot?.artifacts.find((artifact) => artifact.id === id);
    if (!target) return;

    let nextFilter = mapFilter;
    let nextSearch = search;
    const targetIsVisible = filterArtifacts([target], { ...mapFilter, search }).length > 0;

    if (!targetIsVisible) {
      nextFilter = {
        provider: mapFilter.provider && mapFilter.provider !== target.provider
          ? target.provider
          : mapFilter.provider,
        kind: mapFilter.kind && mapFilter.kind !== target.kind
          ? undefined
          : mapFilter.kind,
        scope: mapFilter.scope && mapFilter.scope !== target.scope
          ? undefined
          : mapFilter.scope,
      };
      if (!filterArtifacts([target], { ...nextFilter, search }).length) {
        nextSearch = "";
      }
    }

    const contextChanges = !filtersMatch(mapFilter, nextFilter) || search !== nextSearch;
    if (id !== selectedArtifactId || contextChanges) {
      if (!confirmUnsavedMemoryLoss()) return;
      memoryMutationSequence.current += 1;
      clearLoadedMemory();
    }
    if (contextChanges) {
      setMapFilter(nextFilter);
      setSearch(nextSearch);
    }
    setSelectedArtifactId(id);
  }

  function applyArtifactFilter(nextFilter: MapFilter): boolean {
    if (filtersMatch(mapFilter, nextFilter)) return true;
    if (!confirmUnsavedMemoryLoss()) return false;
    memoryMutationSequence.current += 1;
    setMapFilter(nextFilter);
    setSelectedArtifactId(null);
    clearLoadedMemory();
    return true;
  }

  function applySearch(nextSearch: string) {
    if (search === nextSearch) return;
    if (!confirmUnsavedMemoryLoss()) return;
    memoryMutationSequence.current += 1;
    setSearch(nextSearch);
    setSelectedArtifactId(null);
    clearLoadedMemory();
  }

  useEffect(() => {
    document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
  }, [language]);

  useEffect(() => {
    const handleBeforeUnload = (event: BeforeUnloadEvent) => {
      if (!loadedMemory || loadedMemory.draft === loadedMemory.document.content) return;
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => window.removeEventListener("beforeunload", handleBeforeUnload);
  }, [loadedMemory]);

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

  useEffect(() => {
    if (!tauri || !snapshot?.workspacePath) return;
    const operation = ++snapshotHistorySequence.current;
    setSnapshotHistoryLoading(true);
    setSnapshotHistoryError(null);
    setSnapshotHistoryFeedback(null);
    setSnapshotHistoryWarning(null);
    void listContextSnapshots()
      .then((history) => {
        if (operation === snapshotHistorySequence.current) applySnapshotHistory(history);
      })
      .catch((historyError) => {
        if (operation === snapshotHistorySequence.current) {
          setSnapshotHistoryError(
            historyError instanceof Error ? historyError.message : String(historyError),
          );
        }
      })
      .finally(() => {
        if (operation === snapshotHistorySequence.current) setSnapshotHistoryLoading(false);
      });
  }, [tauri, snapshot?.workspacePath]);

  const filteredArtifacts = useMemo(
    () =>
      snapshot
        ? filterArtifacts(snapshot.artifacts, { ...mapFilter, search })
        : [],
    [snapshot, mapFilter, search],
  );

  const providerFacets = useMemo(
    () => providerFacetCounts(snapshot?.artifacts ?? [], { ...mapFilter, search }),
    [snapshot, mapFilter, search],
  );

  const providerOptions = useMemo(
    () => providerDisplayOrder.filter(
      (provider) => snapshot?.artifacts.some((artifact) => artifact.provider === provider),
    ),
    [snapshot],
  );

  const filteredSnapshot = useMemo<HarnessSnapshot | null>(
    () => snapshot ? { ...snapshot, artifacts: filteredArtifacts } : null,
    [snapshot, filteredArtifacts],
  );

  const visibleArtifactIds = useMemo(
    () => new Set(filteredArtifacts.map((artifact) => artifact.id)),
    [filteredArtifacts],
  );

  const selectedArtifact = useMemo<HarnessArtifact | null>(
    () => snapshot?.artifacts.find((item) => item.id === selectedArtifactId) ?? null,
    [snapshot, selectedArtifactId],
  );

  const counterpartArtifact = useMemo<HarnessArtifact | null>(
    () => selectedArtifact?.counterpartId
      ? snapshot?.artifacts.find((item) => item.id === selectedArtifact.counterpartId) ?? null
      : null,
    [snapshot, selectedArtifact],
  );

  async function handleLoadMemory(reloading = false) {
    if (!tauri || selectedArtifact?.kind !== "memory") return;
    if (reloading && !confirmUnsavedMemoryLoss()) return;
    if (reloading) memoryMutationSequence.current += 1;
    const artifactId = selectedArtifact.id;
    const sequence = ++memoryLoadSequence.current;
    setMemoryLoading(true);
    setMemoryError(null);
    setMemoryFeedback(null);
    try {
      const document = await loadMemoryArtifact(artifactId);
      if (sequence === memoryLoadSequence.current) {
        setLoadedMemory({ document, draft: document.content });
      }
    } catch (loadError) {
      if (sequence === memoryLoadSequence.current) {
        setMemoryError(loadError instanceof Error ? loadError.message : String(loadError));
      }
    } finally {
      if (sequence === memoryLoadSequence.current) setMemoryLoading(false);
    }
  }

  function handleChangeMemoryDraft(content: string) {
    setLoadedMemory((current) => current ? { ...current, draft: content } : current);
    setMemoryError(null);
    setMemoryFeedback(null);
  }

  function handleCancelMemoryChanges() {
    setLoadedMemory((current) => current
      ? { ...current, draft: current.document.content }
      : current);
    setMemoryError(null);
    setMemoryFeedback("discarded");
  }

  async function handleSaveMemory() {
    const memory = loadedMemory;
    const editToken = memory?.document.editToken;
    if (!tauri || !memory || !editToken) return;
    const mutationSequence = ++memoryMutationSequence.current;
    const artifactId = memory.document.artifactId;

    setMemorySaving(true);
    activeMemorySaveSequence.current = mutationSequence;
    setMemoryError(null);
    setMemoryFeedback(null);
    try {
      const result = await saveMemoryArtifact(editToken, memory.draft);
      if (mutationSequence !== memoryMutationSequence.current) return;
      if (!result.saved) {
        setMemoryFeedback("cancelled");
        return;
      }

      // The write is already committed. Do not let later navigation invalidate the
      // local saved state while the best-effort rescan runs.
      setLoadedMemory({
        document: {
          ...memory.document,
          content: memory.draft,
          contentHash: result.contentHash,
          sizeBytes: result.sizeBytes,
          editToken: null,
        },
        draft: memory.draft,
      });
      setMemoryFeedback("saved");

      const refreshSequence = mutationSequence + 1;
      memoryMutationSequence.current = refreshSequence;
      const refreshScanOperation = ++scanSequence.current;
      setScanning(true);
      let nextSnapshot: HarnessSnapshot | null = null;
      try {
        nextSnapshot = await rescanWorkspace();
      } catch {
        if (refreshSequence === memoryMutationSequence.current) {
          setMemoryError(null);
          setMemoryFeedback("savedRefreshFailed");
        }
      } finally {
        if (refreshScanOperation === scanSequence.current) setScanning(false);
      }
      if (!nextSnapshot || refreshSequence !== memoryMutationSequence.current) return;

      if (activeRuntimeWorkspacePath.current !== nextSnapshot.workspacePath) {
        resetRuntimeForWorkspace(nextSnapshot.workspacePath);
      }
      setSnapshot(nextSnapshot);
      setSelectedArtifactId(result.artifactId);
      try {
        const refreshedDocument = await loadMemoryArtifact(result.artifactId);
        if (
          refreshSequence !== memoryMutationSequence.current
          || refreshedDocument.artifactId !== artifactId
        ) return;
        setLoadedMemory({ document: refreshedDocument, draft: refreshedDocument.content });
        setMemoryFeedback("saved");
      } catch {
        if (refreshSequence === memoryMutationSequence.current) {
          setMemoryError(null);
          setMemoryFeedback("savedRefreshFailed");
        }
      }
    } catch (saveError) {
      if (mutationSequence !== memoryMutationSequence.current) return;
      const structuredError = isMemorySaveError(saveError) ? saveError : null;
      if (structuredError?.tokenConsumed) {
        setLoadedMemory((current) => current
          ? {
              ...current,
              document: { ...current.document, editToken: null },
            }
          : current);
      }
      setMemoryError(
        structuredError?.message
        ?? (saveError instanceof Error ? saveError.message : String(saveError)),
      );
    } finally {
      if (activeMemorySaveSequence.current === mutationSequence) {
        activeMemorySaveSequence.current = null;
        setMemorySaving(false);
      }
    }
  }

  const groupedArtifacts = selectedArtifact ? [] : filteredArtifacts;
  const hasUnsavedMemory = Boolean(
    loadedMemory && loadedMemory.draft !== loadedMemory.document.content,
  );
  const inventoryFilterActive = Boolean(
    mapFilter.provider || mapFilter.kind || mapFilter.scope || search.trim(),
  );
  const visibleWarnings = snapshot?.warnings.filter(
    (warning) => !(
      runtimeSnapshot?.state === "connected" &&
      (warning.id === "runtime-not-connected" || warning.id === "observed-not-connected")
    ) && (
      !inventoryFilterActive ||
      warning.artifactIds.length === 0 ||
      warning.artifactIds.some((artifactId) => visibleArtifactIds.has(artifactId))
    ),
  ) ?? [];

  async function handleChooseWorkspace() {
    if (!tauri) return;
    if (!confirmUnsavedMemoryLoss()) return;
    await performScan(() => chooseWorkspace(copy.workspace.chooseDialogTitle), true);
  }

  async function handleCaptureContextSnapshot() {
    if (
      !snapshot
      || snapshotCapturing
      || snapshotHistoryClearing
      || scanning
      || memorySaving
    ) return;
    if (!confirmUnsavedMemoryLoss()) return;
    const runtimeWorkspacePath = snapshot.workspacePath;
    const suspendedRuntimeOperation = tauri
      ? suspendRuntimeForWorkspaceScan()
      : null;
    memoryMutationSequence.current += 1;
    clearLoadedMemory();
    const operation = ++snapshotCaptureSequence.current;
    snapshotHistorySequence.current += 1;
    snapshotComparisonSequence.current += 1;
    setSnapshotComparing(false);
    setSnapshotHistoryLoading(false);
    const previousTargetId = targetCaptureId;
    setSnapshotCapturing(true);
    setSnapshotHistoryError(null);
    setSnapshotHistoryFeedback(null);
    setSnapshotHistoryWarning(null);
    try {
      if (tauri) {
        const result = await captureContextSnapshot();
        if (operation !== snapshotCaptureSequence.current) return;
        resetRuntimeForWorkspace(result.liveSnapshot.workspacePath);
        setSnapshot(result.liveSnapshot);
        setSelectedArtifactId(null);
        setMapFilter({});
        applySnapshotHistory(
          result.history,
          result.captured?.captureId,
          previousTargetId,
        );
        await performRuntimeScan(result.liveSnapshot.workspacePath);
        if (operation !== snapshotCaptureSequence.current) return;
        if (!result.captured) {
          setSnapshotHistoryError(
            result.persistenceError ?? copy.compare.error,
          );
          return;
        }
        const storageWarnings = [
          result.storageStatus.cleanupPending ? copy.compare.cleanupPending : null,
          result.storageStatus.durabilityWarning ? copy.compare.durabilityWarning : null,
        ].filter((message): message is string => Boolean(message));
        setSnapshotHistoryWarning(storageWarnings.join(" ") || null);
      } else {
        const capturedAt = new Date().toISOString();
        const liveSnapshot = { ...snapshot, scannedAt: capturedAt };
        const captureId = `demo-capture-${capturedAt}`;
        const stored = safeStoredSnapshot(
          captureId,
          "demo-snapshot-current",
          liveSnapshot,
          capturedAt,
        );
        if (operation !== snapshotCaptureSequence.current) return;
        syntheticSnapshots.current = [stored, ...syntheticSnapshots.current];
        setSnapshot(liveSnapshot);
        applySnapshotHistory(
          syntheticSnapshots.current.map((item) => item.summary),
          captureId,
          previousTargetId,
        );
      }
      setSnapshotHistoryFeedback("captured");
    } catch (captureError) {
      if (operation !== snapshotCaptureSequence.current) return;
      if (
        suspendedRuntimeOperation === runtimeScanSequence.current
        && activeRuntimeWorkspacePath.current === null
      ) {
        activeRuntimeWorkspacePath.current = runtimeWorkspacePath;
        setRuntimeLoading(false);
      }
      setSnapshotHistoryError(
        captureError instanceof Error ? captureError.message : String(captureError),
      );
    } finally {
      if (operation === snapshotCaptureSequence.current) setSnapshotCapturing(false);
    }
  }

  async function handleInspectContextSnapshot(captureId: string) {
    const operation = ++contextSnapshotLoadSequence.current;
    setContextSnapshotLoading(true);
    setSnapshotHistoryError(null);
    try {
      const result = tauri
        ? await loadContextSnapshot(captureId)
        : syntheticSnapshots.current.find(
            (item) => item.summary.captureId === captureId,
          ) ?? null;
      if (operation === contextSnapshotLoadSequence.current) {
        setInspectedContextSnapshot(result);
      }
    } catch (loadError) {
      if (operation === contextSnapshotLoadSequence.current) {
        setSnapshotHistoryError(loadError instanceof Error ? loadError.message : String(loadError));
      }
    } finally {
      if (operation === contextSnapshotLoadSequence.current) setContextSnapshotLoading(false);
    }
  }

  async function handleCompareContextSnapshots() {
    if (
      !baseCaptureId
      || !targetCaptureId
      || baseCaptureId === targetCaptureId
      || snapshotCapturing
      || snapshotHistoryClearing
    ) return;
    const operation = ++snapshotComparisonSequence.current;
    setSnapshotComparing(true);
    setSnapshotHistoryError(null);
    setSnapshotHistoryFeedback(null);
    try {
      const result = tauri
        ? await compareContextSnapshots(baseCaptureId, targetCaptureId)
        : compareStoredSnapshots(
            syntheticSnapshots.current.find(
              (item) => item.summary.captureId === baseCaptureId,
            )!,
            syntheticSnapshots.current.find(
              (item) => item.summary.captureId === targetCaptureId,
            )!,
          );
      if (operation === snapshotComparisonSequence.current) setSnapshotComparison(result);
    } catch (compareError) {
      if (operation !== snapshotComparisonSequence.current) return;
      setSnapshotHistoryError(
        compareError instanceof Error ? compareError.message : String(compareError),
      );
      setSnapshotComparison(null);
    } finally {
      if (operation === snapshotComparisonSequence.current) setSnapshotComparing(false);
    }
  }

  async function handleClearContextSnapshotHistory() {
    if (
      (!snapshotHistory.length && !(tauri && snapshotHistoryError))
      || snapshotHistoryClearing
      || snapshotCapturing
      || snapshotComparing
    ) return;
    if (!tauri && !window.confirm(copy.compare.clearConfirm)) return;
    setSnapshotHistoryClearing(true);
    snapshotCaptureSequence.current += 1;
    snapshotComparisonSequence.current += 1;
    contextSnapshotLoadSequence.current += 1;
    setSnapshotComparing(false);
    setContextSnapshotLoading(false);
    setSnapshotHistoryError(null);
    setSnapshotHistoryFeedback(null);
    setSnapshotHistoryWarning(null);
    try {
      if (tauri) {
        const result = await clearContextSnapshotHistory();
        if (!result.cleared) return;
      } else {
        syntheticSnapshots.current = [];
      }
      applySnapshotHistory([]);
      setSnapshotHistoryFeedback("cleared");
    } catch (clearError) {
      setSnapshotHistoryError(
        clearError instanceof Error ? clearError.message : String(clearError),
      );
    } finally {
      setSnapshotHistoryClearing(false);
    }
  }

  function handleSelectBaseSnapshot(captureId: string) {
    snapshotComparisonSequence.current += 1;
    setSnapshotComparing(false);
    setBaseCaptureId(captureId);
    setSnapshotComparison(null);
  }

  function handleSelectTargetSnapshot(captureId: string) {
    snapshotComparisonSequence.current += 1;
    setSnapshotComparing(false);
    setTargetCaptureId(captureId);
    setSnapshotComparison(null);
  }

  function handleSwapSnapshots() {
    snapshotComparisonSequence.current += 1;
    setSnapshotComparing(false);
    setBaseCaptureId(targetCaptureId);
    setTargetCaptureId(baseCaptureId);
    setSnapshotComparison(null);
  }

  function selectKind(kind: HarnessKind) {
    if (applyArtifactFilter({ ...mapFilter, kind })) {
      setSection("items");
    }
  }

  const kinds = snapshot
    ? Array.from(new Set(snapshot.artifacts.map((artifact) => artifact.kind))).sort()
    : [];

  const scopes = snapshot
    ? (["user", "repo", "nested", "worktree"] as HarnessScope[]).filter((scope) =>
        snapshot.artifacts.some((artifact) => artifact.scope === scope),
      )
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
                onClick={() => {
                  const preservesMemoryDraft = item.id === "share" || (
                    section === "share"
                    && (item.id === "overview" || item.id === "items")
                  );
                  if (
                    item.id !== section
                    && !preservesMemoryDraft
                    && !confirmUnsavedMemoryLoss()
                  ) return;
                  if (
                    !preservesMemoryDraft
                    && item.id !== "overview"
                    && item.id !== "items"
                  ) {
                    memoryMutationSequence.current += 1;
                    clearLoadedMemory();
                  }
                  setSection(item.id);
                }}
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
                <button
                  key={kind}
                  aria-label={`${copy.labels.kind[kind]} ${count}`}
                  onClick={() => selectKind(kind)}
                >
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
              disabled={scanning || memorySaving || snapshotCapturing || snapshotHistoryClearing}
              >
                <RefreshCw size={15} className={scanning ? "spin" : undefined} />
                {copy.workspace.rescan}
              </button>
            ) : null}
            <button
              className="secondary-button"
              onClick={() => void handleChooseWorkspace()}
              disabled={!tauri || scanning || memorySaving || snapshotCapturing || snapshotHistoryClearing}
            >
              <FolderOpen size={15} /> {copy.workspace.open}
            </button>
          </div>
        </header>

        {error ? <div className="error-banner"><AlertTriangle size={16} />{error}</div> : null}

        <div className="main-scroll">
          {!snapshot ? (
            <EmptyWorkspace language={language} onChoose={() => void handleChooseWorkspace()} />
          ) : section === "share" ? (
            <ShareSnapshot
              snapshot={snapshot}
              language={language}
              synthetic={!tauri}
              hasUnsavedMemory={hasUnsavedMemory}
            />
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
            <SnapshotCompare
              currentSnapshot={snapshot}
              history={snapshotHistory}
              baseCaptureId={baseCaptureId}
              targetCaptureId={targetCaptureId}
              comparison={snapshotComparison}
              inspectedSnapshot={inspectedContextSnapshot}
              loadingHistory={snapshotHistoryLoading}
              capturing={snapshotCapturing}
              captureDisabled={scanning || memorySaving}
              comparing={snapshotComparing}
              loadingSnapshot={contextSnapshotLoading}
              clearing={snapshotHistoryClearing}
              feedback={snapshotHistoryFeedback}
              warning={snapshotHistoryWarning}
              error={snapshotHistoryError}
              language={language}
              synthetic={!tauri}
              onCapture={() => void handleCaptureContextSnapshot()}
              onSelectBase={handleSelectBaseSnapshot}
              onSelectTarget={handleSelectTargetSnapshot}
              onSwap={handleSwapSnapshots}
              onCompare={() => void handleCompareContextSnapshots()}
              onInspect={(captureId) => void handleInspectContextSnapshot(captureId)}
              onClear={() => void handleClearContextSnapshotHistory()}
            />
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
                    counterpartDifferenceCount(snapshot),
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
                        const id = warning.artifactIds.find((artifactId) =>
                          visibleArtifactIds.has(artifactId)
                        ) ?? (inventoryFilterActive ? undefined : warning.artifactIds[0]);
                        if (id) handleSelectArtifact(id);
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
                  <div className="provider-filter">
                    <span className="provider-filter-label">{copy.explorer.providerFilter}</span>
                    <div
                      className="segmented-control"
                      role="group"
                      aria-label={copy.explorer.providerFilter}
                    >
                      <button
                        className={clsx(!mapFilter.provider && "active")}
                        aria-pressed={!mapFilter.provider}
                        aria-label={`${copy.explorer.allProviders} ${providerFacets.total}`}
                        onClick={() => applyArtifactFilter({
                          ...mapFilter,
                          provider: undefined,
                        })}
                      >
                        {copy.explorer.allProviders}
                        <small>{providerFacets.total}</small>
                      </button>
                      {providerOptions.map((provider) => (
                        <button
                          key={provider}
                          className={clsx(mapFilter.provider === provider && "active")}
                          aria-pressed={mapFilter.provider === provider}
                          aria-label={`${copy.labels.provider[provider]} ${providerFacets.byProvider[provider]}`}
                          onClick={() => applyArtifactFilter({ ...mapFilter, provider })}
                        >
                          {copy.labels.provider[provider]}
                          <small>{providerFacets.byProvider[provider]}</small>
                        </button>
                      ))}
                    </div>
                  </div>
                  <div className="toolbar-spacer" />
                  <span className="filter-results" role="status">
                    {copy.explorer.results(filteredArtifacts.length, snapshot.artifacts.length)}
                  </span>
                  <div className="filter-actions">
                    <label className="scope-filter">
                      <span>{copy.explorer.scopeFilter}</span>
                      <select
                        aria-label={copy.explorer.scopeFilter}
                        value={mapFilter.scope ?? ""}
                        onChange={(event) => {
                          const scope = event.target.value as HarnessScope | "";
                          applyArtifactFilter({
                            ...mapFilter,
                            scope: scope || undefined,
                          });
                        }}
                      >
                        <option value="">{copy.explorer.allScopes}</option>
                        {scopes.map((scope) => (
                          <option key={scope} value={scope}>{copy.labels.scope[scope]}</option>
                        ))}
                      </select>
                    </label>
                    {(mapFilter.kind || mapFilter.scope) ? (
                      <button
                        className="filter-chip"
                        onClick={() => applyArtifactFilter({ provider: mapFilter.provider })}
                      >
                        {mapFilter.kind ? copy.labels.kind[mapFilter.kind] : ""}
                        {mapFilter.kind && mapFilter.scope ? " · " : ""}
                        {mapFilter.scope ? copy.labels.scope[mapFilter.scope] : ""}
                        <span>×</span>
                      </button>
                    ) : null}
                    <label className="search-field">
                      <Search size={15} />
                      <input
                        value={search}
                        aria-label={copy.explorer.search}
                        onChange={(event) => applySearch(event.target.value)}
                        placeholder={copy.explorer.search}
                      />
                    </label>
                  </div>
                </div>

                {mode === "map" ? (
                  <HarnessMap
                    snapshot={filteredSnapshot ?? snapshot}
                    language={language}
                    onFilter={(filter) => applyArtifactFilter({ ...mapFilter, ...filter })}
                  />
                ) : (
                  <HarnessTable
                    artifacts={filteredArtifacts}
                    language={language}
                    workspacePath={snapshot.workspacePath}
                    selectedId={selectedArtifactId}
                    onSelect={handleSelectArtifact}
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
          counterpart={counterpartArtifact}
          language={language}
          workspacePath={snapshot?.workspacePath ?? null}
          groupedArtifacts={groupedArtifacts.length === snapshot?.artifacts.length ? [] : groupedArtifacts}
          loadedMemory={loadedMemory}
          memoryLoading={memoryLoading}
          memorySaving={memorySaving}
          memoryError={memoryError}
          memoryFeedback={memoryFeedback}
          canLoadMemory={tauri}
          onSelect={handleSelectArtifact}
          onOpenSource={(path) => {
            if (tauri) void revealSource(path);
          }}
          onLoadMemory={() => void handleLoadMemory()}
          onReloadMemory={() => void handleLoadMemory(true)}
          onChangeMemoryDraft={handleChangeMemoryDraft}
          onCancelMemoryChanges={handleCancelMemoryChanges}
          onSaveMemory={() => void handleSaveMemory()}
        />
      ) : null}
    </div>
  );
}

function isMemorySaveError(value: unknown): value is MemorySaveError {
  return typeof value === "object"
    && value !== null
    && typeof (value as { message?: unknown }).message === "string"
    && typeof (value as { tokenConsumed?: unknown }).tokenConsumed === "boolean";
}
