// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import App from "./App";

afterEach(() => {
  cleanup();
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
});
