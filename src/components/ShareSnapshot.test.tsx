// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { StrictMode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { sampleSnapshot } from "../lib/sample";
import type { CompatibilityReportOutput, HarnessSnapshot } from "../types";
import { ShareSnapshot } from "./ShareSnapshot";

const tauri = vi.hoisted(() => ({
  generateCompatibilityReport: vi.fn(),
}));

vi.mock("../lib/tauri", () => tauri);

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

const output: CompatibilityReportOutput = {
  report: {
    reportSchemaVersion: 1,
    harnessLensVersion: "0.5.0",
    sourceRevision: null,
    sourceDirty: null,
    operatingSystem: "macos",
    architecture: "arm64",
    artifactCount: 9,
    byProvider: { codex: 5, claude: 2, shared: 2 },
    byKind: { instructions: 2, memory: 1 },
    byResolution: { effective: 4, defined: 5 },
    warningCounts: { info: 1, warning: 0, error: 0 },
    scanComplete: true,
    privacyNotice: "Aggregate metadata only.",
  },
  markdown: "# Exact backend Markdown\n\n- Report schema: 1",
  scannedAt: "2026-08-13T08:00:00Z",
};

function renderDesktop(
  snapshot: HarnessSnapshot = sampleSnapshot,
  hasUnsavedMemory = false,
) {
  return render(
    <ShareSnapshot
      snapshot={snapshot}
      language="en"
      synthetic={false}
      hasUnsavedMemory={hasUnsavedMemory}
    />,
  );
}

beforeEach(() => {
  tauri.generateCompatibilityReport.mockReset();
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: vi.fn().mockResolvedValue(undefined) },
  });
});

afterEach(() => cleanup());

describe("Share compatibility report", () => {
  it("generates once, previews the full report, then copies exact backend Markdown explicitly", async () => {
    const pending = deferred<CompatibilityReportOutput>();
    tauri.generateCompatibilityReport.mockReturnValue(pending.promise);
    renderDesktop();

    const generate = screen.getByRole("button", { name: "Rescan & generate report" });
    fireEvent.click(generate);
    fireEvent.click(generate);
    expect(tauri.generateCompatibilityReport).toHaveBeenCalledTimes(1);
    expect(navigator.clipboard.writeText).not.toHaveBeenCalled();

    await act(async () => pending.resolve(output));
    expect(screen.getByLabelText("Complete compatibility report Markdown preview"))
      .toHaveTextContent("Exact backend Markdown");
    expect(navigator.clipboard.writeText).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Copy report" }));
    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(output.markdown);
    });
  });

  it("accepts a report after Strict Mode replays the mount effect", async () => {
    tauri.generateCompatibilityReport.mockResolvedValue(output);
    render(
      <StrictMode>
        <ShareSnapshot
          snapshot={sampleSnapshot}
          language="en"
          synthetic={false}
          hasUnsavedMemory={false}
        />
      </StrictMode>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Rescan & generate report" }));

    expect(await screen.findByLabelText("Complete compatibility report Markdown preview"))
      .toHaveTextContent("Exact backend Markdown");
  });

  it("keeps generation errors non-copyable", async () => {
    tauri.generateCompatibilityReport.mockRejectedValue(new Error("scan unavailable"));
    renderDesktop();

    fireEvent.click(screen.getByRole("button", { name: "Rescan & generate report" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("scan unavailable");
    expect(screen.queryByRole("button", { name: /Copy report|Retry copy/ })).not.toBeInTheDocument();
    expect(navigator.clipboard.writeText).not.toHaveBeenCalled();
  });

  it("retries a failed clipboard write without invoking another scan", async () => {
    tauri.generateCompatibilityReport.mockResolvedValue(output);
    vi.mocked(navigator.clipboard.writeText)
      .mockRejectedValueOnce(new Error("clipboard denied"))
      .mockResolvedValueOnce(undefined);
    renderDesktop();

    fireEvent.click(screen.getByRole("button", { name: "Rescan & generate report" }));
    await screen.findByLabelText("Complete compatibility report Markdown preview");
    fireEvent.click(screen.getByRole("button", { name: "Copy report" }));
    expect(await screen.findByRole("button", { name: "Retry copy" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Retry copy" }));

    await waitFor(() => expect(navigator.clipboard.writeText).toHaveBeenCalledTimes(2));
    expect(tauri.generateCompatibilityReport).toHaveBeenCalledTimes(1);
  });

  it("does not label a newer report copied when an older clipboard write finishes late", async () => {
    const oldCopy = deferred<void>();
    const newerOutput = {
      ...output,
      markdown: "# Newer backend Markdown",
      scannedAt: "2026-08-13T08:01:00Z",
    };
    tauri.generateCompatibilityReport
      .mockResolvedValueOnce(output)
      .mockResolvedValueOnce(newerOutput);
    vi.mocked(navigator.clipboard.writeText).mockReturnValueOnce(oldCopy.promise);
    renderDesktop();

    fireEvent.click(screen.getByRole("button", { name: "Rescan & generate report" }));
    expect(await screen.findByLabelText("Complete compatibility report Markdown preview"))
      .toHaveTextContent("Exact backend Markdown");
    fireEvent.click(screen.getByRole("button", { name: "Copy report" }));
    fireEvent.click(screen.getByRole("button", { name: "Rescan & generate report" }));
    expect(await screen.findByLabelText("Complete compatibility report Markdown preview"))
      .toHaveTextContent("Newer backend Markdown");

    await act(async () => oldCopy.resolve());

    expect(screen.queryByRole("button", { name: "Copied" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Copy report" })).toBeInTheDocument();
  });

  it("explains that an unsaved Memory draft is preserved and excluded", () => {
    renderDesktop(sampleSnapshot, true);

    expect(screen.getByText(/unsaved Memory draft is preserved/i)).toHaveTextContent(
      /reads only the saved disk version/i,
    );
  });

  it("discards a late report when the workspace prop changes", async () => {
    const pending = deferred<CompatibilityReportOutput>();
    tauri.generateCompatibilityReport.mockReturnValue(pending.promise);
    const { rerender } = renderDesktop();
    fireEvent.click(screen.getByRole("button", { name: "Rescan & generate report" }));

    rerender(
      <ShareSnapshot
        snapshot={{ ...sampleSnapshot, workspacePath: "/tmp/other", workspaceName: "other" }}
        language="en"
        synthetic={false}
        hasUnsavedMemory={false}
      />,
    );
    await act(async () => pending.resolve(output));

    expect(screen.queryByLabelText("Complete compatibility report Markdown preview"))
      .not.toBeInTheDocument();
    expect(navigator.clipboard.writeText).not.toHaveBeenCalled();
  });

  it("shows a synthetic v1 example without generation or copying in the browser demo", () => {
    render(
      <ShareSnapshot
        snapshot={sampleSnapshot}
        language="en"
        synthetic
        hasUnsavedMemory={false}
      />,
    );

    expect(screen.getByText("Synthetic · not evidence")).toBeInTheDocument();
    expect(screen.getByLabelText("Synthetic compatibility report example"))
      .toHaveTextContent("SYNTHETIC DEMO");
    expect(screen.queryByRole("button", { name: /generate report/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /copy report/i })).not.toBeInTheDocument();
    expect(tauri.generateCompatibilityReport).not.toHaveBeenCalled();
    expect(navigator.clipboard.writeText).not.toHaveBeenCalled();
  });
});
