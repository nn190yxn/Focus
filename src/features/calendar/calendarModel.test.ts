import { describe, expect, it } from "vitest";

import {
  buildPreviewPeriod,
  dayActivity,
  formatFocusDuration,
  formatPeriodTitle,
  monthCells,
  shiftPeriod,
} from "./calendarModel";

describe("calendarModel", () => {
  it("moves week, month, and year anchors across calendar boundaries", () => {
    expect(shiftPeriod("2026-12-29", "week", 1)).toBe("2027-01-05");
    expect(shiftPeriod("2026-01-31", "month", 1)).toBe("2026-02-01");
    expect(shiftPeriod("2026-07-20", "year", -1)).toBe("2025-01-01");
  });

  it("formats a week that crosses month boundaries", () => {
    expect(formatPeriodTitle("week", "2026-08-01")).toBe("7 月 27 日 - 8 月 2 日");
  });

  it("builds Monday-first month cells including empty dates", () => {
    const cells = monthCells(2026, 7, []);
    expect(cells).toHaveLength(42);
    expect(cells.slice(0, 5)).toEqual([null, null, null, null, null]);
    expect(cells[5]?.date).toBe("2026-08-01");
    expect(cells[35]?.date).toBe("2026-08-31");
    expect(cells[41]).toBeNull();
  });

  it("creates complete zero-inclusive preview periods with activity on the anchor", () => {
    const result = buildPreviewPeriod("month", "2026-07-20");
    expect(result.startsOn).toBe("2026-07-01");
    expect(result.endsOn).toBe("2026-07-31");
    expect(result.days).toHaveLength(31);
    expect(dayActivity(result.days.find((day) => day.date === "2026-07-20")!)).toBe(3);
    expect(formatFocusDuration(3_000)).toBe("50 分钟");
  });
});
