import { invoke } from "@tauri-apps/api/core";
import type {
  CodexRunDetail,
  CodexRuntimeSnapshot,
  ContextSnapshotCaptureResult,
  ContextSnapshotClearResult,
  ContextSnapshotComparison,
  ContextSnapshotSummary,
  HarnessSnapshot,
  MemoryArtifactDocument,
  MemorySaveResult,
  StoredContextSnapshot,
} from "../types";

export function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export async function chooseWorkspace(title: string): Promise<HarnessSnapshot | null> {
  return invoke<HarnessSnapshot | null>("choose_workspace", { title });
}

export async function rescanWorkspace(): Promise<HarnessSnapshot> {
  return invoke<HarnessSnapshot>("rescan_workspace");
}

export async function loadDefaultWorkspace(): Promise<HarnessSnapshot | null> {
  return invoke<HarnessSnapshot | null>("load_default_workspace");
}

export async function revealSource(path: string): Promise<void> {
  await invoke("open_artifact", { path });
}

export async function loadMemoryArtifact(
  artifactId: string,
): Promise<MemoryArtifactDocument> {
  return invoke<MemoryArtifactDocument>("load_memory_artifact", { artifactId });
}

export async function saveMemoryArtifact(
  editToken: string,
  content: string,
): Promise<MemorySaveResult> {
  return invoke<MemorySaveResult>("save_memory_artifact", { editToken, content });
}

export async function inspectRuntime(): Promise<CodexRuntimeSnapshot> {
  return invoke<CodexRuntimeSnapshot>("inspect_runtime");
}

export async function loadRuntimeRun(threadId: string): Promise<CodexRunDetail> {
  return invoke<CodexRunDetail>("load_runtime_run", { threadId });
}

export async function listContextSnapshots(): Promise<ContextSnapshotSummary[]> {
  return invoke<ContextSnapshotSummary[]>("list_context_snapshots");
}

export async function captureContextSnapshot(): Promise<ContextSnapshotCaptureResult> {
  return invoke<ContextSnapshotCaptureResult>("capture_context_snapshot");
}

export async function loadContextSnapshot(
  captureId: string,
): Promise<StoredContextSnapshot> {
  return invoke<StoredContextSnapshot>("load_context_snapshot", { captureId });
}

export async function compareContextSnapshots(
  baseCaptureId: string,
  targetCaptureId: string,
): Promise<ContextSnapshotComparison> {
  return invoke<ContextSnapshotComparison>("compare_context_snapshots", {
    baseCaptureId,
    targetCaptureId,
  });
}

export async function clearContextSnapshotHistory(): Promise<ContextSnapshotClearResult> {
  return invoke<ContextSnapshotClearResult>("clear_context_snapshot_history");
}
