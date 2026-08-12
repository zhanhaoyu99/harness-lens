// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { sampleSnapshot } from "../lib/sample";
import { HarnessMap } from "./HarnessMap";

const { reactFlowMounted } = vi.hoisted(() => ({
  reactFlowMounted: vi.fn(),
}));

vi.mock("@xyflow/react", async () => {
  const React = await vi.importActual<typeof import("react")>("react");
  return {
    Background: () => null,
    BackgroundVariant: { Dots: "dots" },
    Controls: () => null,
    MarkerType: { ArrowClosed: "arrowClosed" },
    MiniMap: () => null,
    Position: { Left: "left", Right: "right" },
    ReactFlow: ({
      children,
      nodes,
      fitView,
    }: {
      children: ReactNode;
      nodes: Array<{ id: string }>;
      fitView?: boolean;
    }) => {
      React.useEffect(() => {
        reactFlowMounted(fitView);
      }, [fitView]);
      return (
        <div data-testid="react-flow" data-node-ids={nodes.map((node) => node.id).join(",")}>
          {children}
        </div>
      );
    },
  };
});

beforeEach(() => {
  reactFlowMounted.mockClear();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("Harness map viewport", () => {
  it("fits again when filtering changes the rendered node set", () => {
    const claudeSnapshot = {
      ...sampleSnapshot,
      artifacts: sampleSnapshot.artifacts.filter((artifact) => artifact.provider === "claude"),
    };
    const { rerender } = render(
      <HarnessMap snapshot={claudeSnapshot} language="en" onFilter={vi.fn()} />,
    );

    const filteredNodeIds = screen.getByTestId("react-flow").getAttribute("data-node-ids");
    expect(reactFlowMounted).toHaveBeenCalledTimes(1);
    expect(reactFlowMounted).toHaveBeenLastCalledWith(true);

    rerender(<HarnessMap snapshot={sampleSnapshot} language="en" onFilter={vi.fn()} />);

    expect(screen.getByTestId("react-flow")).not.toHaveAttribute(
      "data-node-ids",
      filteredNodeIds,
    );
    expect(reactFlowMounted).toHaveBeenCalledTimes(2);

    rerender(
      <HarnessMap
        snapshot={{ ...sampleSnapshot, scannedAt: "2026-08-12T08:00:00Z" }}
        language="en"
        onFilter={vi.fn()}
      />,
    );

    expect(reactFlowMounted).toHaveBeenCalledTimes(2);
  });
});
