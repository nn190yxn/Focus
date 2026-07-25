import { describe, expect, it } from "vitest";

import { summarizeRecurrence, summarizePattern } from "./recurrenceSummary";

describe("recurrence summaries", () => {
  it("summarizes every supported pattern in natural Chinese", () => {
    expect(summarizePattern({ kind: "daily", interval: 1 })).toBe("每天");
    expect(summarizePattern({ kind: "daily", interval: 3 })).toBe("每 3 天");
    expect(summarizePattern({ kind: "weekdays" })).toBe("每个工作日");
    expect(summarizePattern({ kind: "weekly", interval: 2, weekdays: [5, 1] })).toBe("每 2 周的周一、周五");
    expect(summarizePattern({ kind: "monthly", interval: 1, dayOfMonth: 31 })).toBe("每月 31 日");
  });

  it("includes time and effective range in the full summary", () => {
    expect(summarizeRecurrence({ pattern: { kind: "weekdays" }, localTime: "09:30", timezone: "Asia/Shanghai", startsOn: "2026-07-20", endsOn: "2026-08-20" }))
      .toBe("每个工作日 09:30，2026-07-20 至 2026-08-20");
  });
});
