/// <reference types="node" />

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const styles = readFileSync(new URL("../styles.css", import.meta.url), "utf8");

function ruleFor(selector: string): string {
  const escaped = selector.replaceAll(".", "\\.");
  return styles.match(new RegExp(`${escaped}\\s*\\{([^}]*)\\}`))?.[1] ?? "";
}

describe("responsive shell layout contract", () => {
  it("lets the main column shrink so the inner scroller owns overflow", () => {
    expect(ruleFor(".app-frame")).toContain("grid-template-rows: minmax(0, 1fr)");
    expect(ruleFor(".main-column")).toContain("min-height: 0");
    expect(ruleFor(".main-scroll")).toContain("overflow: auto");
  });

  it("keeps the sidebar reachable in short windows", () => {
    expect(ruleFor(".sidebar")).toContain("min-height: 0");
    expect(ruleFor(".sidebar")).toContain("overflow-y: auto");
  });
});
