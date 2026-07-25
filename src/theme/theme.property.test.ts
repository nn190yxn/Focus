import fc from "fast-check";
import { describe, expect, it } from "vitest";

import { colorModes, resolveThemeTokens, themeNames } from "./theme";

describe("P10 shared theme consistency", () => {
  it("resolves identical semantic tokens for the main window and widget", () => {
    fc.assert(
      fc.property(
        fc.constantFrom(...themeNames),
        fc.constantFrom(...colorModes),
        (theme, mode) => {
          expect(resolveThemeTokens("main", theme, mode)).toEqual(
            resolveThemeTokens("widget", theme, mode),
          );
        },
      ),
      { numRuns: 100 },
    );
  });
});
