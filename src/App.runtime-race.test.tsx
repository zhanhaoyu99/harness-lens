// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { sampleSnapshot } from "./lib/sample";
import type {
  CodexRuntimeSnapshot,
  ContextSnapshotSummary,
  HarnessSnapshot,
} from "./types";

const tauri = vi.hoisted(() => ({
  captureContextSnapshot: vi.fn(),
  chooseWorkspace: vi.fn(),
  clearContextSnapshotHistory: vi.fn(),
  compareContextSnapshots: vi.fn(),
  generateCompatibilityReport: vi.fn(),
  inspectRuntime: vi.fn(),
  isTauriRuntime: vi.fn(),
  listContextSnapshots: vi.fn(),
  loadContextSnapshot: vi.fn(),
  loadDefaultWorkspace: vi.fn(),
  loadMemoryArtifact: vi.fn(),
  loadRuntimeRun: vi.fn(),
  rescanWorkspace: vi.fn(),
  revealSource: vi.fn(),
  saveMemoryArtifact: vi.fn(),
}));

vi.mock("./lib/tauri", () => tauri);

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason: unknown) => void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

function workspace(name: string): HarnessSnapshot {
  return {
    ...sampleSnapshot,
    workspacePath: `/tmp/${name}`,
    workspaceName: name,
    scannedAt: `2026-08-13T0${name === "workspace-a" ? "1" : "2"}:00:00Z`,
  };
}

function runtime(label: string): CodexRuntimeSnapshot {
  return {
    state: "connected",
    codexVersion: `codex-${label}`,
    observedAt: "2026-08-13T03:00:00Z",
    message: null,
    skills: [],
    hooks: [],
    runs: [
      {
        id: `run-${label}`,
        title: `${label} run`,
        preview: label,
        status: "completed",
        source: "test",
        createdAt: null,
        updatedAt: null,
        parentThreadId: null,
        gitBranch: null,
      },
    ],
  };
}

function captureSummary(snapshot: HarnessSnapshot): ContextSnapshotSummary {
  return {
    captureId: "capture-current",
    snapshotId: "a".repeat(64),
    schemaVersion: 1,
    workspaceKey: "b".repeat(64),
    workspaceName: snapshot.workspaceName,
    gitBranch: snapshot.gitBranch,
    capturedAt: snapshot.scannedAt,
    itemCount: snapshot.artifacts.length,
    diagnosticCount: snapshot.warnings.length,
    complete: true,
    appVersion: "0.4.0",
    scannerVersion: "0.4.0",
  };
}

async function openInitialWorkspace(
  initialWorkspace: HarnessSnapshot,
  initialRuntime: CodexRuntimeSnapshot,
) {
  tauri.chooseWorkspace.mockResolvedValueOnce(initialWorkspace);
  tauri.inspectRuntime.mockResolvedValueOnce(initialRuntime);
  render(<App />);

  await waitFor(() => expect(tauri.loadDefaultWorkspace).toHaveBeenCalledTimes(1));
  fireEvent.click(screen.getByRole("button", { name: "Choose workspace" }));
  await screen.findByText(initialWorkspace.workspaceName);
  fireEvent.click(screen.getByRole("button", { name: "Runs" }));
  await screen.findByText(initialRuntime.runs[0].title);
}

beforeEach(() => {
  for (const mock of Object.values(tauri)) mock.mockReset();
  tauri.isTauriRuntime.mockReturnValue(true);
  tauri.loadDefaultWorkspace.mockResolvedValue(null);
  tauri.listContextSnapshots.mockResolvedValue([]);
  tauri.clearContextSnapshotHistory.mockResolvedValue({ cleared: true });
});

afterEach(() => {
  cleanup();
});

describe("runtime scan workspace isolation", () => {
  it("keeps an unsaved Memory draft when navigating to and from Share", async () => {
    const workspaceA = workspace("workspace-a");
    tauri.chooseWorkspace.mockResolvedValueOnce(workspaceA);
    tauri.inspectRuntime.mockResolvedValueOnce(runtime("initial-a"));
    tauri.loadMemoryArtifact.mockResolvedValueOnce({
      artifactId: "memory",
      editToken: "memory-edit-token",
      content: "saved Memory content",
      contentHash: "c".repeat(64),
      sizeBytes: 20,
      editable: true,
      editabilityReason: null,
    });
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
    render(<App />);

    await waitFor(() => expect(tauri.loadDefaultWorkspace).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByRole("button", { name: "Choose workspace" }));
    await screen.findByText(workspaceA.workspaceName);
    fireEvent.click(screen.getByRole("row", { name: "Inspect Memory registry" }));
    fireEvent.click(screen.getByRole("button", { name: "View memory content" }));
    const editor = await screen.findByRole("textbox", { name: "Memory content" });
    fireEvent.change(editor, { target: { value: "unsaved Memory draft" } });

    fireEvent.click(screen.getByRole("button", { name: "Share" }));
    expect(screen.getByRole("heading", { name: "Compatibility Report" })).toBeInTheDocument();
    expect(screen.getByText(/unsaved Memory draft is preserved/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Harness Items" }));

    expect(confirm).not.toHaveBeenCalled();
    expect(screen.getByRole("textbox", { name: "Memory content" })).toHaveValue(
      "unsaved Memory draft",
    );
  });

  it("keeps an A refresh from overwriting B data or ending B loading", async () => {
    const workspaceA = workspace("workspace-a");
    const workspaceB = workspace("workspace-b");
    const staleRefresh = deferred<CodexRuntimeSnapshot>();
    const workspaceSwitch = deferred<HarnessSnapshot | null>();
    const currentRefresh = deferred<CodexRuntimeSnapshot>();

    await openInitialWorkspace(workspaceA, runtime("initial-a"));
    tauri.inspectRuntime.mockReturnValueOnce(staleRefresh.promise);
    fireEvent.click(screen.getByRole("button", { name: "Refresh runtime" }));
    await waitFor(() => expect(tauri.inspectRuntime).toHaveBeenCalledTimes(2));

    tauri.chooseWorkspace.mockReturnValueOnce(workspaceSwitch.promise);
    tauri.inspectRuntime.mockReturnValueOnce(currentRefresh.promise);
    fireEvent.click(screen.getByRole("button", { name: "Open workspace" }));

    await act(async () => {
      staleRefresh.resolve(runtime("stale-a"));
      await staleRefresh.promise;
    });
    expect(screen.getByRole("button", { name: "Refresh runtime" })).toBeDisabled();
    expect(screen.queryByText("stale-a run")).not.toBeInTheDocument();

    await act(async () => {
      workspaceSwitch.resolve(workspaceB);
      await workspaceSwitch.promise;
    });
    await screen.findByText(workspaceB.workspaceName);
    await waitFor(() => expect(tauri.inspectRuntime).toHaveBeenCalledTimes(3));

    await act(async () => {
      currentRefresh.resolve(runtime("current-b"));
      await currentRefresh.promise;
    });
    await screen.findByText("current-b run");
    expect(screen.getByRole("button", { name: "Refresh runtime" })).toBeEnabled();
    expect(screen.queryByText("stale-a run")).not.toBeInTheDocument();
  });

  it("keeps an old refresh failure from polluting capture runtime state", async () => {
    const workspaceA = workspace("workspace-a");
    const capturedWorkspace = {
      ...workspaceA,
      scannedAt: "2026-08-13T04:00:00Z",
    };
    const staleRefresh = deferred<CodexRuntimeSnapshot>();
    const capture = deferred<{
      liveSnapshot: HarnessSnapshot;
      captured: ContextSnapshotSummary;
      history: ContextSnapshotSummary[];
      persistenceError: null;
      storageStatus: {
        cleanupPending: false;
        cleanupWarning: null;
        durabilityWarning: null;
      };
    }>();
    const captureRefresh = deferred<CodexRuntimeSnapshot>();
    const summary = captureSummary(capturedWorkspace);

    await openInitialWorkspace(workspaceA, runtime("initial-a"));
    tauri.inspectRuntime.mockReturnValueOnce(staleRefresh.promise);
    fireEvent.click(screen.getByRole("button", { name: "Refresh runtime" }));
    await waitFor(() => expect(tauri.inspectRuntime).toHaveBeenCalledTimes(2));

    tauri.captureContextSnapshot.mockReturnValueOnce(capture.promise);
    tauri.inspectRuntime.mockReturnValueOnce(captureRefresh.promise);
    fireEvent.click(screen.getByRole("button", { name: "Compare" }));
    fireEvent.click(screen.getByRole("button", { name: "Capture snapshot" }));
    fireEvent.click(screen.getByRole("button", { name: "Runs" }));

    await act(async () => {
      staleRefresh.reject(new Error("stale runtime failure"));
      await staleRefresh.promise.catch(() => undefined);
    });
    expect(screen.getByRole("button", { name: "Refresh runtime" })).toBeDisabled();
    expect(screen.queryByText("stale runtime failure")).not.toBeInTheDocument();

    await act(async () => {
      capture.resolve({
        liveSnapshot: capturedWorkspace,
        captured: summary,
        history: [summary],
        persistenceError: null,
        storageStatus: {
          cleanupPending: false,
          cleanupWarning: null,
          durabilityWarning: null,
        },
      });
      await capture.promise;
    });
    await waitFor(() => expect(tauri.inspectRuntime).toHaveBeenCalledTimes(3));

    await act(async () => {
      captureRefresh.resolve(runtime("capture-current"));
      await captureRefresh.promise;
    });
    await screen.findByText("capture-current run");
    expect(screen.getByRole("button", { name: "Refresh runtime" })).toBeEnabled();
    expect(screen.queryByText("stale runtime failure")).not.toBeInTheDocument();
  });
});
