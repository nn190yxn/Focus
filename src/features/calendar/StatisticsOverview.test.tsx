// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { buildPreviewPeriod } from "./calendarModel";
import { StatisticsOverview } from "./StatisticsOverview";
import { buildStatisticsSummary } from "./statisticsModel";

describe("StatisticsOverview", () => {
  it("shows summary metrics, trend, and project investment", () => {
    render(<StatisticsOverview summary={buildStatisticsSummary(buildPreviewPeriod("month", "2026-07-20"))} />);

    expect(screen.getByText("计划完成率")).toBeInTheDocument();
    expect(screen.getByText("50", { selector: ".statistics-metrics strong" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "完成与专注趋势" })).toBeInTheDocument();
    expect(screen.getByText("抵达 Focus")).toBeInTheDocument();
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("offers a focus entry when the selected period has no activity", () => {
    const onStartFocus = vi.fn();
    const summary = buildStatisticsSummary({
      period: "week",
      startsOn: "2026-07-20",
      endsOn: "2026-07-20",
      days: [{ date: "2026-07-20", plannedTasks: [], completedTasks: [], focusSessions: [] }],
      projects: [],
    });
    render(<StatisticsOverview summary={summary} onStartFocus={onStartFocus} />);

    fireEvent.click(screen.getByRole("button", { name: "开始一轮专注" }));
    expect(onStartFocus).toHaveBeenCalledOnce();
  });
});
