// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Inspector, type LoadedMemoryState } from "./Inspector";
import { sampleSnapshot } from "../lib/sample";

afterEach(cleanup);

const memoryArtifact = sampleSnapshot.artifacts.find((item) => item.kind === "memory")!;

function renderInspector(options: {
  canLoadMemory?: boolean;
  loadedMemory?: LoadedMemoryState | null;
  onLoadMemory?: () => void;
  onSaveMemory?: () => void;
  onCancelMemoryChanges?: () => void;
} = {}) {
  render(
    <Inspector
      artifact={memoryArtifact}
      counterpart={null}
      warnings={sampleSnapshot.warnings}
      language="zh"
      workspacePath={sampleSnapshot.workspacePath}
      groupedArtifacts={[]}
      loadedMemory={options.loadedMemory ?? null}
      memoryLoading={false}
      memorySaving={false}
      memoryError={null}
      memoryFeedback={null}
      canLoadMemory={options.canLoadMemory ?? true}
      onSelect={vi.fn()}
      onOpenSource={vi.fn()}
      onLoadMemory={options.onLoadMemory ?? vi.fn()}
      onReloadMemory={vi.fn()}
      onChangeMemoryDraft={vi.fn()}
      onCancelMemoryChanges={options.onCancelMemoryChanges ?? vi.fn()}
      onSaveMemory={options.onSaveMemory ?? vi.fn()}
    />,
  );
}

describe("Memory inspector", () => {
  it("does not load raw Memory content before an explicit click", () => {
    const onLoadMemory = vi.fn();
    renderInspector({ onLoadMemory });

    expect(screen.queryByRole("textbox", { name: "记忆正文" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "查看记忆正文" }));
    expect(onLoadMemory).toHaveBeenCalledOnce();
  });

  it("shows an explicit editor and save controls for an editable Memory file", () => {
    const onSaveMemory = vi.fn();
    const onCancelMemoryChanges = vi.fn();
    renderInspector({
      loadedMemory: {
        document: {
          artifactId: memoryArtifact.id,
          editToken: "opaque-token",
          content: "# Original memory",
          contentHash: "a".repeat(64),
          sizeBytes: 17,
          editable: true,
          editabilityReason: null,
        },
        draft: "# Edited memory",
      },
      onSaveMemory,
      onCancelMemoryChanges,
    });

    expect(screen.getByRole("textbox", { name: "记忆正文" })).toHaveValue("# Edited memory");
    fireEvent.click(screen.getByRole("button", { name: "保存记忆" }));
    fireEvent.click(screen.getByRole("button", { name: "放弃修改" }));
    expect(onSaveMemory).toHaveBeenCalledOnce();
    expect(onCancelMemoryChanges).toHaveBeenCalledOnce();
  });

  it("makes the browser demo boundary explicit", () => {
    renderInspector({ canLoadMemory: false });

    expect(screen.getByText(/浏览器合成演示不会读取或编辑本地记忆文件/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "查看记忆正文" })).not.toBeInTheDocument();
  });
});

describe("cross-tool comparison diagnostic", () => {
  it("explains that a same-name difference is not an effective-state error", () => {
    const artifact = sampleSnapshot.artifacts.find((item) => item.id === "skill-qa-codex")!;
    const counterpart = sampleSnapshot.artifacts.find((item) => item.id === artifact.counterpartId)!;
    const onSelect = vi.fn();

    render(
      <Inspector
        artifact={artifact}
        counterpart={counterpart}
        warnings={sampleSnapshot.warnings}
        language="zh"
        workspacePath={sampleSnapshot.workspacePath}
        groupedArtifacts={[]}
        loadedMemory={null}
        memoryLoading={false}
        memorySaving={false}
        memoryError={null}
        memoryFeedback={null}
        canLoadMemory={false}
        onSelect={onSelect}
        onOpenSource={vi.fn()}
        onLoadMemory={vi.fn()}
        onReloadMemory={vi.fn()}
        onChangeMemoryDraft={vi.fn()}
        onCancelMemoryChanges={vi.fn()}
        onSaveMemory={vi.fn()}
      />,
    );

    expect(screen.getByText(/这个对照信号不会改变任一条目的生效状态/)).toBeInTheDocument();
    expect(screen.getByText("Claude · 项目级")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "查看对照条目" }));
    expect(onSelect).toHaveBeenCalledWith(counterpart.id);
  });
});

describe("artifact position and deterministic diagnostics", () => {
  it("shows a generated purpose and qualifies the 200-line rule as a heuristic", () => {
    const artifact = {
      ...sampleSnapshot.artifacts.find((item) => item.id === "rule-repo")!,
      lineCount: 237,
    };

    render(
      <Inspector
        artifact={artifact}
        counterpart={null}
        warnings={[{
          id: "quality:guidance-line-review",
          severity: "info",
          title: "Long guidance",
          detail: "Review it",
          artifactIds: [artifact.id],
        }]}
        language="zh"
        workspacePath={sampleSnapshot.workspacePath}
        groupedArtifacts={[]}
        loadedMemory={null}
        memoryLoading={false}
        memorySaving={false}
        memoryError={null}
        memoryFeedback={null}
        canLoadMemory={false}
        onSelect={vi.fn()}
        onOpenSource={vi.fn()}
        onLoadMemory={vi.fn()}
        onReloadMemory={vi.fn()}
        onChangeMemoryDraft={vi.fn()}
        onCancelMemoryChanges={vi.fn()}
        onSaveMemory={vi.fn()}
      />,
    );

    expect(screen.getByText("Codex · 项目级 · 指令")).toBeInTheDocument();
    expect(screen.getByText("根据条目类型生成；提供方与作用域单独作为定位展示")).toBeInTheDocument();
    expect(screen.getByText("规范较长 · 237 行")).toBeInTheDocument();
    expect(screen.getByText(/只是可维护性启发式规则/)).toBeInTheDocument();
    expect(screen.getByText(/不是 AI 自动评审、质量评分、成功率预测/)).toBeInTheDocument();
  });
});
