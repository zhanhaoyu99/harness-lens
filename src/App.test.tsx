// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("Harness source filters", () => {
  it("separates providers and composes the source lens with kind filters", () => {
    render(<App />);

    const sourceFilter = screen.getByRole("group", { name: "Harness source" });
    expect(within(sourceFilter).getByRole("button", { name: "All 9" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(within(sourceFilter).queryByRole("button", { name: /Plugin/ })).not.toBeInTheDocument();

    fireEvent.click(within(sourceFilter).getByRole("button", { name: "Claude 2" }));

    expect(screen.getByRole("status")).toHaveTextContent("2 / 9 items");
    expect(screen.getByRole("row", { name: "Inspect CLAUDE.md" })).toBeInTheDocument();
    expect(screen.queryByRole("row", { name: "Inspect Global AGENTS.md" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Agents 2" }));

    expect(screen.getByRole("status")).toHaveTextContent("1 / 9 items");
    expect(within(sourceFilter).getByRole("button", { name: "Claude 1" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    const claudeAgent = screen.getByRole("row", { name: "Inspect qa" });
    expect(claudeAgent).toBeInTheDocument();

    fireEvent.click(claudeAgent);
    fireEvent.click(screen.getByRole("button", { name: "View comparison item" }));

    expect(within(sourceFilter).getByRole("button", { name: "Codex 1" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("status")).toHaveTextContent("1 / 9 items");
    expect(screen.getByRole("row", { name: "Inspect qa" })).toHaveTextContent("Codex");
  });

  it("loads saved snapshot metadata without replacing the live inventory", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Compare" }));
    expect(screen.getByRole("heading", { name: "Snapshot History & Compare" })).toBeInTheDocument();

    const savedSnapshots = screen.getAllByRole("button", { name: /Inspect saved metadata/ });
    fireEvent.click(savedSnapshots[savedSnapshots.length - 1]);
    fireEvent.click(screen.getByRole("button", { name: "Overview" }));

    expect(screen.getByRole("row", { name: "Inspect unattended-issue-dev" })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("9 / 9 items");
  });

  it("keeps synthetic history when clear confirmation is cancelled", () => {
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Compare" }));
    expect(screen.getAllByRole("button", { name: /Inspect saved metadata/ })).toHaveLength(3);
    fireEvent.click(screen.getByRole("button", { name: "Clear history" }));

    expect(confirm).toHaveBeenCalledWith(
      "Permanently clear the saved snapshot history for this workspace?",
    );
    expect(screen.getAllByRole("button", { name: /Inspect saved metadata/ })).toHaveLength(3);
  });
});
