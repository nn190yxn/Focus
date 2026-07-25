import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const css = readFileSync(new URL("./global.css", import.meta.url), "utf8");

function rule(selector: string) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = css.match(new RegExp(`${escaped}\\s*\\{([^}]+)\\}`));
  expect(match, `Missing CSS rule: ${selector}`).not.toBeNull();
  return match?.[1] ?? "";
}

describe("memo desktop layout CSS contracts", () => {
  it("uses a fixed list column and a flexible editor column", () => {
    const workspace = rule(".memo-workspace");
    expect(workspace).toContain("grid-template-columns: 360px minmax(0, 1fr)");
    expect(workspace).toContain("overflow: hidden");
  });

  it("keeps independent scrolling inside both panes", () => {
    const panes = rule(".memo-list-pane, .memo-editor");
    expect(panes).toContain("min-height: 0");
    expect(panes).toContain("overflow-y: auto");
    expect(panes).toContain("overscroll-behavior: contain");
  });

  it("switches to retained single-pane views below 980 pixels", () => {
    expect(rule(".memo-workspace")).toContain("container-type: inline-size");
    const narrowLayout = css.match(/@container \(max-width: 979px\)\s*\{([\s\S]*?)\n\}/)?.[1] ?? "";
    expect(narrowLayout).toContain("grid-template-columns: minmax(0, 1fr)");
    expect(narrowLayout).toContain("[data-mobile-view=\"list\"] .memo-editor");
    expect(narrowLayout).toContain("[data-mobile-view=\"editor\"] .memo-list-pane");
    expect(narrowLayout).toContain(".memo-back-button { display: inline-flex");
  });
});

describe("memo list item CSS contracts", () => {
  it("clamps the body preview to two lines and exposes compact metadata", () => {
    expect(rule(".memo-list-item__preview")).toContain("-webkit-line-clamp: 2");
    expect(rule(".memo-list-item__tags")).toContain("display: flex");
    expect(css).toMatch(/\.memo-list-item__meta\s*\{\s*margin-top:\s*auto/);
  });

  it("keeps memo controls usable at text zoom and narrow widths", () => {
    expect(rule(".memo-tag-filters button")).toContain("min-height: 40px");
    expect(rule(".memo-tags-editor__items button")).toContain("width: 40px");
    expect(rule(".memo-tags-editor__items button")).toContain("height: 40px");
    expect(rule(".memo-reminder-editor .segmented button")).toContain("min-height: 40px");
    expect(rule(".memo-reminder-weekdays label")).toContain("min-height: 40px");
    const narrowLayout = css.match(/@container \(max-width: 979px\)\s*\{([\s\S]*?)\n\}/)?.[1] ?? "";
    expect(narrowLayout).toContain(".memo-reminder-fields { grid-template-columns: minmax(0, 1fr)");
  });
});
