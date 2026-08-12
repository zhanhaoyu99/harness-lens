import { invoke } from "@tauri-apps/api/core";
import type {
  CodexRunDetail,
  CodexRuntimeSnapshot,
  HarnessSnapshot,
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

export async function inspectRuntime(): Promise<CodexRuntimeSnapshot> {
  return invoke<CodexRuntimeSnapshot>("inspect_runtime");
}

export async function loadRuntimeRun(threadId: string): Promise<CodexRunDetail> {
  return invoke<CodexRunDetail>("load_runtime_run", { threadId });
}
