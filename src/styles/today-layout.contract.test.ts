import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const css = readFileSync(new URL("./global.css", import.meta.url), "utf8");

function rule(selector: string) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = css.match(new RegExp(`${escaped}\\s*\\{([^}]+)\\}`));
  expect(match, `Missing CSS rule: ${selector}`).not.toBeNull();
  return match?.[1] ?? "";
}

describe("today workspace layout CSS contracts", () => {
  it("keeps the desktop columns aligned with the design baseline", () => {
    expect(rule(".today-grid")).toContain("grid-template-columns: 260px minmax(360px, 1fr) 300px");
    expect(rule(".day-column")).toContain("min-width: 0");
  });

  it("provides a readable, scrollable long-form note editor", () => {
    const panel = rule(".note-panel");
    const textarea = rule(".note-panel textarea");

    expect(panel).toContain("min-height: 380px");
    expect(panel).toContain("overflow: visible");
    expect(textarea).toContain("min-height: 240px");
    expect(textarea).toContain("max-height: 55vh");
    expect(textarea).toContain("overflow-y: auto");
    expect(textarea).toContain("resize: vertical");
    expect(textarea).toContain("font-size: 14px");
  });

  it("retains a full-width single-column note on narrow screens", () => {
    const narrowLayout = css.match(/@media \(max-width: 820px\)\s*\{([\s\S]*?)\n\}/)?.[1] ?? "";

    expect(narrowLayout).toContain(".today-grid { grid-template-columns: 1fr");
    expect(narrowLayout).toContain(".goal-panel, .day-column { grid-column: auto");
  });
});
