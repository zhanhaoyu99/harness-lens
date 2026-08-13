// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  sampleContextSnapshotHistory,
  sampleSnapshot,
  sampleStoredContextSnapshots,
} from "../lib/sample";
import { compareStoredSnapshots } from "../lib/snapshots";
import { SnapshotCompare } from "./SnapshotCompare";

afterEach(cleanup);

describe("SnapshotCompare", () => {
  it("keeps formal comparison saved-to-saved and filters overlapping signals", () => {
    renderCompare();

    const baseline = screen.getByRole("combobox", { name: "Baseline" });
    expect(within(baseline).getAllByRole("option")).toHaveLength(
      sampleContextSnapshotHistory.length + 1,
    );
    expect(within(baseline).queryByRole("option", { name: /Current scan/ })).not.toBeInTheDocument();
    expect(screen.getByText("Current scan (not saved)")).toBeInTheDocument();

    expect(screen.getByText("Distinct changed items").previousSibling).toHaveTextContent("4");
    const filters = screen.getByRole("group", { name: "Snapshot change filters" });
    fireEvent.click(within(filters).getByRole("button", { name: /Content/ }));

    expect(screen.getAllByText("Project AGENTS.md").length).toBeGreaterThan(0);
    expect(screen.queryByText("Legacy agent config")).not.toBeInTheDocument();
    expect(screen.getByText(/line-level diff is not retained/)).toBeInTheDocument();
  });

  it("routes explicit capture, inspect, compare, swap and clear actions", () => {
    const actions = {
      onCapture: vi.fn(),
      onInspect: vi.fn(),
      onCompare: vi.fn(),
      onSwap: vi.fn(),
      onClear: vi.fn(),
    };
    renderCompare(actions);

    fireEvent.click(screen.getByRole("button", { name: "Capture snapshot" }));
    fireEvent.click(screen.getAllByRole("button", { name: /Inspect saved metadata/ })[0]);
    fireEvent.click(screen.getByRole("button", { name: "Compare saved snapshots" }));
    fireEvent.click(screen.getByRole("button", { name: "Swap" }));
    fireEvent.click(screen.getByRole("button", { name: "Clear history" }));

    expect(actions.onCapture).toHaveBeenCalledOnce();
    expect(actions.onInspect).toHaveBeenCalledWith(sampleContextSnapshotHistory[0].captureId);
    expect(actions.onCompare).toHaveBeenCalledOnce();
    expect(actions.onSwap).toHaveBeenCalledOnce();
    expect(actions.onClear).toHaveBeenCalledOnce();
  });

  it("qualifies absence counts and filters when either saved scan was incomplete", () => {
    const comparison = {
      ...compareStoredSnapshots(
        sampleStoredContextSnapshots[1],
        sampleStoredContextSnapshots[0],
      ),
      complete: false,
    };
    render(
      <SnapshotCompare
        currentSnapshot={sampleSnapshot}
        history={sampleContextSnapshotHistory}
        baseCaptureId={sampleContextSnapshotHistory[1].captureId}
        targetCaptureId={sampleContextSnapshotHistory[0].captureId}
        comparison={comparison}
        inspectedSnapshot={sampleStoredContextSnapshots[0]}
        loadingHistory={false}
        capturing={false}
        captureDisabled={false}
        comparing={false}
        loadingSnapshot={false}
        clearing={false}
        feedback={null}
        error={null}
        language="en"
        synthetic
        onCapture={vi.fn()}
        onSelectBase={vi.fn()}
        onSelectTarget={vi.fn()}
        onSwap={vi.fn()}
        onCompare={vi.fn()}
        onInspect={vi.fn()}
        onClear={vi.fn()}
      />,
    );

    expect(screen.getAllByText("Only in target").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Only in baseline").length).toBeGreaterThan(0);
    expect(screen.queryByText("Added")).not.toBeInTheDocument();
    expect(screen.queryByText("Removed")).not.toBeInTheDocument();
  });

  it("keeps clear-history recovery available when stored history is unreadable", () => {
    render(
      <SnapshotCompare
        currentSnapshot={sampleSnapshot}
        history={[]}
        baseCaptureId=""
        targetCaptureId=""
        comparison={null}
        inspectedSnapshot={null}
        loadingHistory={false}
        capturing={false}
        captureDisabled={false}
        comparing={false}
        loadingSnapshot={false}
        clearing={false}
        feedback={null}
        error="The saved history is corrupted."
        language="en"
        synthetic={false}
        onCapture={vi.fn()}
        onSelectBase={vi.fn()}
        onSelectTarget={vi.fn()}
        onSwap={vi.fn()}
        onCompare={vi.fn()}
        onInspect={vi.fn()}
        onClear={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: "Clear history" })).toBeEnabled();
  });
});

function renderCompare(overrides: Partial<{
  onCapture: () => void;
  onInspect: (captureId: string) => void;
  onCompare: () => void;
  onSwap: () => void;
  onClear: () => void;
}> = {}) {
  const comparison = compareStoredSnapshots(
    sampleStoredContextSnapshots[1],
    sampleStoredContextSnapshots[0],
  );
  return render(
    <SnapshotCompare
      currentSnapshot={sampleSnapshot}
      history={sampleContextSnapshotHistory}
      baseCaptureId={sampleContextSnapshotHistory[1].captureId}
      targetCaptureId={sampleContextSnapshotHistory[0].captureId}
      comparison={comparison}
      inspectedSnapshot={sampleStoredContextSnapshots[0]}
      loadingHistory={false}
      capturing={false}
      captureDisabled={false}
      comparing={false}
      loadingSnapshot={false}
      clearing={false}
      feedback={null}
      error={null}
      language="en"
      synthetic
      onCapture={overrides.onCapture ?? vi.fn()}
      onSelectBase={vi.fn()}
      onSelectTarget={vi.fn()}
      onSwap={overrides.onSwap ?? vi.fn()}
      onCompare={overrides.onCompare ?? vi.fn()}
      onInspect={overrides.onInspect ?? vi.fn()}
      onClear={overrides.onClear ?? vi.fn()}
    />,
  );
}
