import { describe, expect, it } from "vitest";
import {
  localizeResolutionReason,
  localizeWarning,
  messages,
  resolveInitialLanguage,
} from "./i18n";
import type { HarnessWarning } from "../types";

describe("i18n", () => {
  it("follows the primary system language when no preference is stored", () => {
    expect(resolveInitialLanguage(null, ["zh-CN", "en-US"])).toBe("zh");
    expect(resolveInitialLanguage(null, ["en-US", "zh-CN"])).toBe("en");
  });

  it("prefers a persisted language over the system language", () => {
    expect(resolveInitialLanguage("en", ["zh-CN"])).toBe("en");
    expect(resolveInitialLanguage("zh", ["en-US"])).toBe("zh");
  });

  it("provides localized labels and generic runtime warnings", () => {
    const warning: HarnessWarning = {
      id: "runtime-not-connected",
      severity: "info",
      title: "Runtime evidence is not connected yet",
      detail: "Defined and effective states come from static adapter rules.",
      artifactIds: [],
    };

    expect(messages.zh.labels.scope.user).toBe("用户全局");
    expect(messages.zh.labels.scope.repo).toBe("项目级");
    expect(messages.zh.labels.scope.nested).toBe("子项目级");
    expect(messages.zh.labels.scope.worktree).toBe("项目绑定");
    expect(localizeWarning(warning, "zh").title).toBe("尚未连接运行时证据");
  });

  it("localizes built-in resolution reasons without changing unknown content", () => {
    expect(
      localizeResolutionReason("Loaded as the global Codex instruction source.", "zh"),
    ).toBe("作为全局 Codex 指令源加载。");
    expect(localizeResolutionReason("Custom project explanation", "zh")).toBe(
      "Custom project explanation",
    );
  });
});
