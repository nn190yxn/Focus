import { describe, expect, it } from "vitest";

import { buildPreviewPeriod } from "./calendarModel";
import { buildStatisticsSummary, trendBuckets } from "./statisticsModel";
import type { CalendarPeriodResult } from "./types";

describe("statisticsModel", () => {
  it("aggregates preview activity into stable summary metrics", () => {
    const summary = buildStatisticsSummary(buildPreviewPeriod("month", "2026-07-20"));

    expect(summary.completedTaskCount).toBe(1);
    expect(summary.focusMinutes).toBe(50);
    expect(summary.effectiveSessionCount).toBe(1);
    expect(summary.activeDayCount).toBe(1);
    expect(summary.projectInvestments).toEqual([
      expect.objectContaining({ focusSeconds: 3_000, focusPercent: 100 }),
    ]);
    expect(summary.trend).toHaveLength(31);
  });

  it("returns zeros while retaining every trend date for an empty period", () => {
    const calendar: CalendarPeriodResult = {
      period: "week",
      startsOn: "2026-07-20",
      endsOn: "2026-07-21",
      days: ["2026-07-20", "2026-07-21"].map((date) => ({
        date,
        plannedTasks: [],
        completedTasks: [],
        focusSessions: [],
      })),
      projects: [],
    };

    const summary = buildStatisticsSummary(calendar);
    expect(summary.completionPercent).toBe(0);
    expect(summary.focusMinutes).toBe(0);
    expect(summary.activeDayCount).toBe(0);
    expect(summary.trend).toHaveLength(2);
  });

  it("groups a year trend into monthly buckets", () => {
    const summary = buildStatisticsSummary(buildPreviewPeriod("year", "2026-07-20"));
    const buckets = trendBuckets(summary);

    expect(buckets).toHaveLength(12);
    expect(buckets[0].label).toBe("1月");
    expect(buckets[11].label).toBe("12月");
    expect(buckets.reduce((total, bucket) => total + bucket.focusSeconds, 0)).toBe(summary.focusSeconds);
  });
});
