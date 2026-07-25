import type { CalendarPeriodResult } from "./types";
import type { ProjectInvestment, StatisticsSummary, StatisticsTrendBucket } from "./statisticsTypes";

export function buildStatisticsSummary(calendar: CalendarPeriodResult): StatisticsSummary {
  const plannedTaskCount = calendar.days.reduce((total, day) => total + day.plannedTasks.length, 0);
  const completedPlannedCount = calendar.days.reduce(
    (total, day) => total + day.plannedTasks.filter((task) => task.status === "completed").length,
    0,
  );
  const completedTaskCount = calendar.days.reduce((total, day) => total + day.completedTasks.length, 0);
  const focusSeconds = calendar.days.reduce(
    (total, day) => total + day.focusSessions.reduce((dayTotal, session) => dayTotal + session.actualSeconds, 0),
    0,
  );
  const effectiveSessionCount = calendar.days.reduce((total, day) => total + day.focusSessions.length, 0);
  const projectInvestment = new Map<string, ProjectInvestment>();

  for (const session of calendar.days.flatMap((day) => day.focusSessions)) {
    if (!session.project) continue;
    const current = projectInvestment.get(session.project.id) ?? {
      project: session.project,
      focusSeconds: 0,
      effectiveSessionCount: 0,
      focusPercent: 0,
    };
    current.focusSeconds += session.actualSeconds;
    current.effectiveSessionCount += 1;
    projectInvestment.set(session.project.id, current);
  }

  const projectInvestments = [...projectInvestment.values()]
    .map((investment) => ({
      ...investment,
      focusPercent: percentage(investment.focusSeconds, focusSeconds),
    }))
    .sort((left, right) => right.focusSeconds - left.focusSeconds || left.project.name.localeCompare(right.project.name));

  return {
    period: calendar.period,
    startsOn: calendar.startsOn,
    endsOn: calendar.endsOn,
    plannedTaskCount,
    completedTaskCount,
    completionPercent: percentage(completedPlannedCount, plannedTaskCount),
    focusSeconds,
    focusMinutes: Math.floor(focusSeconds / 60),
    effectiveSessionCount,
    activeDayCount: calendar.days.filter((day) => day.completedTasks.length > 0 || day.focusSessions.length > 0).length,
    trend: calendar.days.map((day) => ({
      date: day.date,
      plannedTaskCount: day.plannedTasks.length,
      completedTaskCount: day.completedTasks.length,
      focusSeconds: day.focusSessions.reduce((total, session) => total + session.actualSeconds, 0),
      effectiveSessionCount: day.focusSessions.length,
    })),
    projectInvestments,
  };
}

export function trendBuckets(summary: StatisticsSummary, locale = "zh-CN"): StatisticsTrendBucket[] {
  if (summary.period !== "year") {
    return summary.trend.map((point) => ({ ...point, label: String(Number(point.date.slice(8, 10))) }));
  }

  const buckets = new Map<string, StatisticsTrendBucket>();
  for (const point of summary.trend) {
    const month = point.date.slice(0, 7);
    const current = buckets.get(month) ?? {
      date: month,
      label: new Intl.DateTimeFormat(locale, { month: "short" }).format(new Date(Number(month.slice(0, 4)), Number(month.slice(5, 7)) - 1, 1, 12)),
      plannedTaskCount: 0,
      completedTaskCount: 0,
      focusSeconds: 0,
      effectiveSessionCount: 0,
    };
    current.plannedTaskCount += point.plannedTaskCount;
    current.completedTaskCount += point.completedTaskCount;
    current.focusSeconds += point.focusSeconds;
    current.effectiveSessionCount += point.effectiveSessionCount;
    buckets.set(month, current);
  }
  return [...buckets.values()];
}

function percentage(part: number, total: number): number {
  return total > 0 ? Math.floor((part * 100) / total) : 0;
}
