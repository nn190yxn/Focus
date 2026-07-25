import { describe, expect, it } from "vitest";

import { colorModes, resolveColorMode, resolveThemeTokens, themeNames } from "./theme";

function relativeLuminance(oklch: string) {
  const match = oklch.match(/^oklch\(([\d.]+) ([\d.]+) ([\d.]+)\)$/);
  if (!match) throw new Error(`Unsupported color: ${oklch}`);
  const lightness = Number(match[1]);
  const chroma = Number(match[2]);
  const hue = Number(match[3]) * Math.PI / 180;
  const a = chroma * Math.cos(hue);
  const b = chroma * Math.sin(hue);
  const l = (lightness + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (lightness - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (lightness - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const red = Math.min(1, Math.max(0, 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s));
  const green = Math.min(1, Math.max(0, -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s));
  const blue = Math.min(1, Math.max(0, -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s));
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function contrastRatio(first: string, second: string) {
  const lighter = Math.max(relativeLuminance(first), relativeLuminance(second));
  const darker = Math.min(relativeLuminance(first), relativeLuminance(second));
  return (lighter + 0.05) / (darker + 0.05);
}

describe("resolveColorMode", () => {
  it("resolves system appearance from the current platform preference", () => {
    expect(resolveColorMode("system", false)).toBe("light");
    expect(resolveColorMode("system", true)).toBe("dark");
  });

  it("keeps an explicit appearance independent of the platform preference", () => {
    expect(resolveColorMode("light", true)).toBe("light");
    expect(resolveColorMode("dark", false)).toBe("dark");
  });
});

describe("theme contrast", () => {
  it.each(themeNames.flatMap((theme) => colorModes.map((mode) => [theme, mode] as const)))(
    "keeps %s %s text combinations at WCAG AA contrast",
    (theme, mode) => {
      const tokens = resolveThemeTokens("main", theme, mode);
      const combinations = [
        [tokens.text, tokens.canvas],
        [tokens.text, tokens.surface],
        [tokens.textMuted, tokens.canvas],
        [tokens.textMuted, tokens.surface],
        [tokens.accentStrong, tokens.surface],
        [tokens.accentContrast, tokens.accentStrong],
        [tokens.success, tokens.surface],
        [tokens.warning, tokens.surface],
        [tokens.danger, tokens.surface],
      ];

      for (const [foreground, background] of combinations) {
        expect(contrastRatio(foreground, background)).toBeGreaterThanOrEqual(4.5);
      }
    },
  );
});
