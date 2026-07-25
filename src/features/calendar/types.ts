export type CalendarPeriod = "week" | "month" | "year";
export type CalendarSourceKind = "task" | "recurringInstance";
export type CalendarTaskStatus = "pending" | "completed" | "skipped" | "rescheduled";
export type CalendarCompletionKind = "deadline" | "early";

export type CalendarProject = {
  id: string;
  name: string;
  color: string;
  icon: string;
  status: string;
};

export type CalendarTaskItem = {
  sourceKind: CalendarSourceKind;
  sourceId: string;
  title: string;
  category: string;
  project: CalendarProject | null;
  scheduledDate: string | null;
  scheduledTime: string | null;
  status: CalendarTaskStatus;
  completedAt: string | null;
};

export type CalendarFocusSession = {
  id: string;
  title: string;
  category: string | null;
  project: CalendarProject | null;
  actualSeconds: number;
  completionKind: CalendarCompletionKind;
  startedAt: string;
  endedAt: string;
};

export type CalendarDay = {
  date: string;
  plannedTasks: CalendarTaskItem[];
  completedTasks: CalendarTaskItem[];
  focusSessions: CalendarFocusSession[];
};

export type CalendarPeriodResult = {
  period: CalendarPeriod;
  startsOn: string;
  endsOn: string;
  days: CalendarDay[];
  projects: CalendarProject[];
};

export type CalendarQuery = {
  period: CalendarPeriod;
  anchorDate: string;
  timezone: string;
  category: string | null;
  projectId: string | null;
};
