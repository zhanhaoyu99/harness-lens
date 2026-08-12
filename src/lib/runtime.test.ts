import { describe, expect, it } from "vitest";
import { sampleRunDetail } from "./sample";
import {
  buildTurnReplay,
  formatDuration,
  formatRuntimeStatus,
  formatStepKind,
} from "./runtime";

describe("runtime replay helpers", () => {
  it("keeps normalized evidence in turn order", () => {
    const detail = sampleRunDetail("demo-thread-release-gate");
    expect(detail).not.toBeNull();

    const replay = buildTurnReplay(detail!);

    expect(replay).toHaveLength(2);
    expect(replay[0].steps.map((step) => step.id)).toEqual([
      "demo-step-request",
      "demo-step-reasoning",
      "demo-step-search",
      "demo-step-agent",
    ]);
    expect(replay[1].steps).toHaveLength(4);
  });

  it("localizes known runtime vocabulary and preserves unknown values", () => {
    expect(formatRuntimeStatus("completed", "zh")).toBe("已完成");
    expect(formatRuntimeStatus("custom", "zh")).toBe("custom");
    expect(
      formatStepKind({ kind: "fileChange", label: "File changes" }, "zh"),
    ).toBe("文件变更");
    expect(formatStepKind({ kind: "custom", label: "Custom event" }, "zh")).toBe(
      "Custom event",
    );
  });

  it("formats duration without implying cost or task success", () => {
    expect(formatDuration(65_000, "en")).toBe("1m 5s");
    expect(formatDuration(65_000, "zh")).toBe("1 分 5 秒");
    expect(formatDuration(null, "en")).toBe("—");
  });
});
