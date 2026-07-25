import type { CalendarPeriod, CalendarProject } from "./types";

export type StatisticsTrendPoint = {
  date: string;
  plannedTaskCount: number;
  completedTaskCount: number;
  focusSeconds: number;
  effectiveSessionCount: number;
};

export type ProjectInvestment = {
  project: CalendarProject;
  focusSeconds: number;
  effectiveSessionCount: number;
  focusPercent: number;
};

export type StatisticsSummary = {
  period: CalendarPeriod;
  startsOn: string;
  endsOn: string;
  plannedTaskCount: number;
  completedTaskCount: number;
  completionPercent: number;
  focusSeconds: number;
  focusMinutes: number;
  effectiveSessionCount: number;
  activeDayCount: number;
  trend: StatisticsTrendPoint[];
  projectInvestments: ProjectInvestment[];
};

export type StatisticsTrendBucket = StatisticsTrendPoint & {
  label: string;
};
