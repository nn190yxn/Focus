import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const css = readFileSync(new URL("./global.css", import.meta.url), "utf8");

function rule(selector: string) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = css.match(new RegExp(`${escaped}\\s*\\{([^}]+)\\}`));
  expect(match, `Missing CSS rule: ${selector}`).not.toBeNull();
  return match?.[1] ?? "";
}

describe("accessibility CSS contracts", () => {
  it("keeps a visible semantic focus ring on keyboard-focusable controls", () => {
    const focusRule = rule(":where(button, input, select, textarea, a[href], [tabindex]):focus-visible");

    expect(focusRule).toContain("outline: 3px solid var(--color-focus-ring)");
    expect(focusRule).toContain("outline-offset: 3px");
  });

  it("disables decorative motion for the reduced-motion preference", () => {
    const media = css.match(/@media \(prefers-reduced-motion: reduce\)\s*\{([\s\S]*?)\n\}/)?.[1] ?? "";

    expect(media).toContain("scroll-behavior: auto !important");
    expect(media).toContain("transition: none !important");
    expect(media).toContain("animation: none !important");
  });

  it("retains reflow and scrolling boundaries under text zoom", () => {
    expect(rule("html, body, #root")).toContain("min-width: 320px");
    expect(rule(".main-content")).toContain("min-width: 0");
    expect(rule(".sidebar")).toContain("overflow-y: auto");
    expect(rule(".dialog")).toContain("overflow: auto");
    expect(rule(".widget")).toContain("overflow: auto");

    const narrowLayout = css.match(/@media \(max-width: 560px\)\s*\{([\s\S]*?)\n\}/)?.[1] ?? "";
    expect(narrowLayout).toContain("flex-wrap: wrap");
  });
});
