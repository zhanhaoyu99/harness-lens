// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { sampleSnapshot } from "../lib/sample";
import { HarnessTable } from "./HarnessTable";

afterEach(cleanup);

describe("Harness table positioning and diagnostics", () => {
  it("keeps purpose and review hints visible without claiming a health score", () => {
    const instructions = {
      ...sampleSnapshot.artifacts.find((item) => item.id === "rule-repo")!,
      lineCount: 237,
    };
    const skill = sampleSnapshot.artifacts.find((item) => item.id === "skill-verify")!;
    const onSelect = vi.fn();

    render(
      <HarnessTable
        artifacts={[instructions, skill]}
        warnings={[{
          id: "quality:guidance-line-review",
          severity: "info",
          title: "Long guidance",
          detail: "Review it",
          artifactIds: [instructions.id],
        }]}
        language="zh"
        workspacePath={sampleSnapshot.workspacePath}
        selectedId={null}
        onSelect={onSelect}
      />,
    );

    expect(screen.getByRole("columnheader", { name: "定位与功能" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "诊断" })).toBeInTheDocument();
    expect(screen.getByText("Codex · 项目级 · 指令")).toBeInTheDocument();
    expect(screen.getByText("规范较长 · 237 行")).toBeInTheDocument();
    expect(screen.getByText("当前规则未发现提示")).toBeInTheDocument();
    expect(screen.getByText(/Run device acceptance and collect evidence/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("row", { name: "查看 Project AGENTS.md" }));
    expect(onSelect).toHaveBeenCalledWith(instructions.id);
  });
});
